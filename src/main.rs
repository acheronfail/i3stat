use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::process;
use std::rc::Rc;
use std::time::Duration;

use clap::Parser;
use staturs::cli::Cli;
use staturs::config;
use staturs::context::{Context, SharedState};
use staturs::dbus::{dbus_subscribe, DbusInterest};
use staturs::dispatcher::Dispatcher;
use staturs::i3::header::I3BarHeader;
use staturs::i3::ipc::handle_click_events;
use staturs::i3::I3Item;
use staturs::ipc::handle_ipc_events;
use staturs::signals::handle_signals;
use tokio::sync::mpsc;

fn main() {
    if let Err(err) = start_runtime() {
        log::error!("{}", err);
        process::exit(1);
    }
}

fn start_runtime() -> Result<Infallible, Box<dyn Error>> {
    pretty_env_logger::try_init()?;

    let args = Cli::parse();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let result = tokio::task::LocalSet::new().block_on(&runtime, async_main(args));

    // NOTE: we use tokio's stdin implementation which spawns a background thread and blocks,
    // we have to shutdown the runtime ourselves here. If we didn't, then when the runtime is
    // dropped it would block indefinitely until that background thread unblocked (i.e., another
    // JSON line from i3).
    // Thus, if anything other than the stdin task fails, we have to manually shut it down here.
    // See: https://github.com/tokio-rs/tokio/discussions/5684
    runtime.shutdown_timeout(Duration::from_secs(1));

    result
}

async fn async_main(args: Cli) -> Result<Infallible, Box<dyn Error>> {
    let config = config::read(args.config).await?;

    println!("{}", serde_json::to_string(&I3BarHeader::default())?);
    println!("[");

    let item_count = config.items.len();

    // shared context
    let state = SharedState::new();

    // state for the bar (moved to bar_printer)
    let bar: Rc<RefCell<_>> = Rc::new(RefCell::new(vec![I3Item::empty(); item_count]));
    let mut bar_txs = vec![];
    let mut dbus_interests = vec![];

    // for each BarItem, spawn a new task to manage it
    let (item_tx, mut item_rx) = mpsc::channel(item_count + 1);
    for (idx, item) in config.items.iter().enumerate() {
        let bar_item = item.to_bar_item();
        bar_item
            .register_dbus_interest()
            .map(|(bus, rule)| dbus_interests.push(DbusInterest::new(idx, bus, rule)));

        let (event_tx, event_rx) = mpsc::channel(32);
        bar_txs.push(event_tx.clone());

        let ctx = Context::new(state.clone(), item_tx.clone(), event_tx, event_rx, idx);
        let bar = bar.clone();
        tokio::task::spawn_local(async move {
            let theme = ctx.theme.clone();
            let fut = bar_item.start(ctx);
            // since this item has terminated, remove its entry from the bar
            match fut.await {
                Ok(()) => {
                    log::info!("item[{}] finished running", idx);
                    // replace with an empty item
                    bar.borrow_mut()[idx] = I3Item::empty();
                }
                Err(e) => {
                    log::error!("item[{}] exited with error: {}", idx, e);
                    // replace with an error item
                    bar.borrow_mut()[idx] = I3Item::new("ERROR")
                        .color(theme.dark1)
                        .background_color(theme.error);
                }
            }
        });
    }

    let dispatcher = Dispatcher::new(
        bar_txs
            .into_iter()
            .enumerate()
            .map(|(idx, tx)| (idx, (tx, config.items[idx].clone())))
            .collect::<HashMap<usize, _>>(),
    );

    if let Err(e) = dbus_subscribe(dispatcher.clone(), dbus_interests).await {
        log::error!("dbus subscription failed: {}", e);
    }

    // task to manage updating the bar and printing it as JSON
    tokio::task::spawn_local(async move {
        while let Some((item, i)) = item_rx.recv().await {
            let mut bar = bar.borrow_mut();
            // always override the bar item's `instance`, since we track that ourselves
            bar[i] = item.instance(i.to_string());
            // print bar to STDOUT for i3
            match serde_json::to_string(&*bar) {
                Ok(json) => println!("{},", json),
                Err(e) => {
                    log::error!("failed to serialise bar to json: {}", e);
                    println!(
                        r#"[{{"full_text":"FATAL ERROR: see logs in stderr","color":"black","background":"red"}}],"#
                    );
                }
            }
        }
    });

    // handle incoming signals
    let signal_handle = handle_signals(args.socket.clone(), dispatcher.clone())?;

    // handle our inputs: i3's IPC and our own IPC
    let err = tokio::select! {
        err = handle_ipc_events(args.socket.clone(), dispatcher.clone()) => err,
        err = handle_click_events(dispatcher.clone()) => err,
    };

    // if we reach here, then something went wrong while reading STDIN, so clean up
    signal_handle.close();
    return err;
}
