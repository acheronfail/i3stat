use std::collections::HashMap;

use libc::{SIGRTMAX, SIGRTMIN, SIGTERM};
use signal_hook_tokio::{Handle, Signals};

use crate::config::AppConfig;
use crate::context::BarEvent;
use crate::dispatcher::Dispatcher;
use crate::error::Result;
use crate::util::RcCell;

// NOTE: the `signal_hook` crate isn't designed to be used with realtime signals, because
// they may be lost due to its internal buffering, etc. For our use case, I think this is
// fine as is, but if not, we may have to use `signal_hook_register` to do it ourselves.
// See: https://docs.rs/signal-hook/latest/signal_hook/index.html#limitations
pub fn handle_signals(config: RcCell<AppConfig>, dispatcher: RcCell<Dispatcher>) -> Result<Handle> {
    let min = SIGRTMIN();
    let max = SIGRTMAX();
    let realtime_signals = min..=max;

    let mut sig_to_indices: HashMap<i32, Vec<usize>> = HashMap::new();
    for (idx, item) in config.items.iter().enumerate() {
        if let Some(sig) = item.common.signal {
            // signals are passed in from 0..(SIGRTMAX - SIGRTMIN)
            let translated_sig = min + sig as i32;
            // make sure all signals are valid
            if !realtime_signals.contains(&translated_sig) {
                bail!(
                    "Invalid signal: {}. Valid signals range from 0 up to {} inclusive",
                    sig,
                    max - min
                );
            }

            log::debug!(
                "mapping signal {} ({}) to item: {} ({})",
                sig,
                translated_sig,
                idx,
                item.name()
            );
            sig_to_indices
                .entry(translated_sig)
                .and_modify(|v| v.push(idx))
                .or_insert_with(|| vec![idx]);
        }
    }

    let mut signals = Signals::new(realtime_signals.chain([SIGTERM]))?;
    let handle = signals.handle();
    let socket_path = config.socket();
    tokio::task::spawn_local(async move {
        use futures::stream::StreamExt;

        loop {
            match signals.next().await {
                None => break,
                // when i3 kills its status_command, it sends SIGTERM, so handle that and clean up
                Some(SIGTERM) => {
                    let _ = std::fs::remove_file(&socket_path);
                    std::process::exit(0);
                }
                // any other signal will be a realtime signal
                Some(signal) => {
                    // find all items which are listening for this signal
                    match sig_to_indices.get(&signal) {
                        // send signal event to all items
                        Some(indices) => {
                            for idx in indices {
                                if let Err(e) =
                                    dispatcher.send_bar_event(*idx, BarEvent::Signal).await
                                {
                                    log::warn!("failed to send signal: {}", e);
                                    continue;
                                }
                            }
                        }
                        None => {
                            log::warn!(
                                "received signal: SIGRTMIN+{} but no item is expecting it",
                                signal - min
                            );
                            continue;
                        }
                    }
                }
            }
        }

        log::error!("unexpected end of signal stream, can no longer handle signals");
    });

    Ok(handle)
}
