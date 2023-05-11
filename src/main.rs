#![feature(exclusive_range_pattern)]

mod bar_items;
mod cli;
mod config;
mod context;
mod exec;
mod format;
pub mod i3;
mod theme;

use std::cell::RefCell;
use std::convert::Infallible;
use std::error::Error;
use std::process;
use std::rc::Rc;

use clap::Parser;
use libc::{SIGRTMAX, SIGRTMIN};
use signal_hook_tokio::{Handle, Signals};
use tokio::io::{stdin, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{self, Sender};

use crate::cli::Cli;
use crate::config::Common;
use crate::context::{Context, SharedState};
use crate::i3::click::I3ClickEvent;
use crate::i3::header::I3BarHeader;
use crate::i3::I3Item;

macro_rules! cont {
    ($($arg:tt)*) => {{
        log::warn!($($arg)*);
        continue;
    }};
}

#[derive(Debug)]
pub enum BarEvent {
    Click(I3ClickEvent),
    Signal,
}

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

    tokio::task::LocalSet::new().block_on(&runtime, async_main(args))
}

async fn async_main(args: Cli) -> Result<Infallible, Box<dyn Error>> {
    let config = config::read(args.config).await?;

    println!("{}", serde_json::to_string(&I3BarHeader::default())?);
    println!("[");

    let item_count = config.items.len();
    let (item_common, items): (Vec<_>, Vec<_>) =
        config.items.into_iter().map(|i| i.to_bar_item()).unzip();

    // shared context
    let state = SharedState::new();

    // state for the bar (moved to bar_printer)
    let bar: Rc<RefCell<_>> = Rc::new(RefCell::new(vec![i3::I3Item::empty(); item_count]));
    let mut bar_txs: Vec<Sender<BarEvent>> = Vec::with_capacity(item_count);

    // for each BarItem, spawn a new task to manage it
    let (item_tx, mut item_rx) = mpsc::channel(item_count + 1);
    for (idx, bar_item) in items.into_iter().enumerate() {
        let (event_tx, event_rx) = mpsc::channel(32);
        bar_txs.push(event_tx);
        let ctx = Context::new(state.clone(), item_tx.clone(), event_rx, idx);
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

    // task to manage updating the bar and printing it as JSON
    // TODO: buffer these and only print a single line within a threshold (no point in super quick updates)
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
    let signal_handle = handle_signals(bar_txs.clone(), item_common.clone())?;

    // IPC click event loop from i3
    let err = handle_click_events(bar_txs).await;

    // if we reach here, then something went wrong while reading STDIN, so clean up
    signal_handle.close();
    return err;
}

// NOTE: the `signal_hook` crate isn't designed to be used with realtime signals, because
// they may be lost due to its internal buffering, etc. For our use case, I think this is
// fine as is, but if not, we may have to use `signal_hook_register` to do it ourselves.
// See: https://docs.rs/signal-hook/latest/signal_hook/index.html#limitations
fn handle_signals(
    bar_tx: Vec<Sender<BarEvent>>,
    mut item_common: Vec<Common>,
) -> Result<Handle, Box<dyn Error>> {
    let min = SIGRTMIN();
    let max = SIGRTMAX();
    let realtime_signals = min..=max;

    // make sure all signals are valid
    for common in item_common.iter_mut() {
        if let Some(sig) = common.signal.as_mut() {
            // signals are passed in from 0..(SIGRTMAX - SIGRTMIN)
            let translated_sig = min + *sig as i32;
            // check this is a valid realtime signal
            match realtime_signals.contains(&translated_sig) {
                true => {
                    // update signal to be the actual signal number
                    *sig = translated_sig as u32;
                }
                false => {
                    return Err(format!(
                        "Invalid signal: {}. Valid signals range from 0 up to {} inclusive",
                        translated_sig,
                        max - min
                    )
                    .into())
                }
            }
        }
    }

    let mut signals = Signals::new(realtime_signals)?;
    let handle = signals.handle();
    tokio::task::spawn_local(async move {
        use futures::stream::StreamExt;

        while let Some(signal) = signals.next().await {
            // find all items which are listening for this signal
            let indices: Vec<usize> = item_common
                .iter()
                .enumerate()
                .filter_map(|(idx, c)| {
                    if c.signal == Some(signal as u32) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();

            if indices.is_empty() {
                cont!(
                    "received signal: SIGRTMIN+{} but no item is expecting it",
                    signal - min
                );
            }

            // send signal event to all items
            for idx in indices {
                if let Err(e) = send_bar_event(&bar_tx[idx], BarEvent::Signal).await {
                    cont!("failed to send signal event to item[{}]: {}", idx, e);
                }
            }
        }
    });

    Ok(handle)
}

/// This task should
async fn handle_click_events(bar_tx: Vec<Sender<BarEvent>>) -> Result<Infallible, Box<dyn Error>> {
    let s = BufReader::new(stdin());
    let mut lines = s.lines();
    loop {
        let mut line = lines
            .next_line()
            .await?
            .ok_or_else(|| "received unexpected end of STDIN")?;

        // skip opening array as part of the protocol
        if line.trim() == "[" {
            continue;
        }

        // skip over any preceding `,` as part of the protocol
        line = line
            .chars()
            .skip_while(|c| c.is_whitespace() || *c == ',')
            .collect();

        // parse click event (single line JSON)
        let click = serde_json::from_str::<I3ClickEvent>(&line)?;

        // parse bar item index from the "instance" property
        let i = match click.instance.as_ref() {
            Some(inst) => match inst.parse::<usize>() {
                Ok(i) => i,
                Err(e) => cont!(
                    "failed to parse click 'instance' property: {}, error: {}",
                    inst,
                    e
                ),
            },
            None => cont!(
                "received click event without 'instance' property, cannot route to item: {:?}",
                click
            ),
        };

        if let Err(e) = send_bar_event(&bar_tx[i], BarEvent::Click(click)).await {
            cont!("failed to send click event to item[{}]: {}", i, e);
        }
    }
}

async fn send_bar_event(tx: &mpsc::Sender<BarEvent>, ev: BarEvent) -> Result<(), Box<dyn Error>> {
    // if the channel fills up (the bar never reads click events), since this is a bounded channel
    // sending the event would block forever, so just drop the event
    if tx.capacity() == 0 {
        return Err("dropping event (channel is full)".into());
    }

    // send click event to its corresponding bar item
    if let Err(SendError(_)) = tx.send(ev).await {
        return Err("dropping event (receiver dropped)".into());
    }

    Ok(())
}
