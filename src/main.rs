#![feature(exclusive_range_pattern)]

mod bar_items;
mod config;
mod context;
mod exec;
pub mod i3;
mod theme;

use std::convert::Infallible;
use std::error::Error;

use libc::{SIGRTMAX, SIGRTMIN};
use signal_hook_tokio::{Handle, Signals};
use tokio::io::{stdin, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::context::{Context, SharedState};
use crate::i3::click::I3ClickEvent;
use crate::i3::header::I3BarHeader;

macro_rules! json {
    ($input:expr) => {
        serde_json::to_string(&$input).unwrap()
    };
}

macro_rules! cont {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        continue;
    }};
}

#[derive(Debug)]
pub enum BarEvent {
    Click(I3ClickEvent),
    Signal,
}

// TODO: central place for storing formatting options? (precision, GB vs G, padding, etc)
// TODO: logging facilities for errors, etc

fn main() -> Result<Infallible, Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    tokio::task::LocalSet::new().block_on(&runtime, async move { async_main().await })
}

async fn async_main() -> Result<Infallible, Box<dyn Error>> {
    let config = config::read().await?;

    println!("{}", json!(I3BarHeader::default()));
    println!("[");

    let item_count = config.items.len();
    let items = config
        .items
        .iter()
        .map(|i| i.to_bar_item())
        .collect::<Vec<Box<dyn context::BarItem>>>();

    // shared context
    let state = SharedState::new();

    // state for the bar (moved to bar_printer)
    let mut bar: Vec<i3::I3Item> = vec![i3::I3Item::empty(); item_count];
    let mut bar_tx: Vec<mpsc::Sender<BarEvent>> = Vec::with_capacity(item_count);

    // for each BarItem, spawn a new task to manage it
    let (item_tx, mut item_rx) = mpsc::channel(item_count);
    for (i, bar_item) in items.into_iter().enumerate() {
        let (event_tx, event_rx) = mpsc::channel(32);
        bar_tx.push(event_tx);
        let ctx = Context::new(state.clone(), item_tx.clone(), event_rx, i);
        tokio::task::spawn_local(async move {
            let fut = bar_item.start(ctx);
            // TODO: handle if a bar item fails
            fut.await.unwrap();
        });
    }

    // task to manage updating the bar and printing it as JSON
    // TODO: buffer these and only print a single line within a threshold (no point in super quick updates)
    tokio::spawn(async move {
        while let Some((item, i)) = item_rx.recv().await {
            // always override the bar item's `instance`, since we track that ourselves
            bar[i] = item.instance(i.to_string());
            println!("{},", json!(bar));
        }
    });

    // handle incoming signals
    let signal_handle = handle_signals(bar_tx.clone())?;

    // IPC click event loop from i3
    let err = handle_click_events(bar_tx).await;

    // if we reach here, then something went wrong while reading STDIN, so clean up
    signal_handle.close();
    return err;
}

// NOTE: the `signal_hook` crate isn't designed to be used with realtime signals, because
// they may be lost due to its internal buffering, etc. For our use case, I think this is
// fine as is, but if not, we may have to use `signal_hook_register` to do it ourselves.
// See: https://docs.rs/signal-hook/latest/signal_hook/index.html#limitations
fn handle_signals(bar_tx: Vec<mpsc::Sender<BarEvent>>) -> Result<Handle, Box<dyn Error>> {
    // TODO: error if a signal is requested that's outside this range
    let min = SIGRTMIN();
    let max = SIGRTMAX();
    let signals = (min..=max).collect::<Vec<_>>();
    let mut signals = Signals::new(signals)?;

    let handle = signals.handle();
    tokio::task::spawn_local(async move {
        use futures::stream::StreamExt;

        while let Some(signal) = signals.next().await {
            // TODO: setup a way (config?) to map a signal to an item, not hardcoded...
            if signal == min + 4 {
                send_bar_event(&bar_tx[8], BarEvent::Signal).await.unwrap();
            } else {
                eprintln!("Received signal: {}", signal);
            }
        }
    });

    Ok(handle)
}

/// This task should
async fn handle_click_events(
    bar_tx: Vec<mpsc::Sender<BarEvent>>,
) -> Result<Infallible, Box<dyn Error>> {
    let s = BufReader::new(stdin());
    let mut lines = s.lines();
    loop {
        let mut line = lines
            .next_line()
            .await?
            .ok_or_else(|| "Received unexpected end of STDIN")?;

        // skip opening array as part of the protocol
        if line.trim() == "[" {
            continue;
        }

        // skip over any preceding `,` as part of the protocol
        line = line.chars().skip_while(|c| c.is_whitespace() || *c == ',').collect();

        // parse click event (single line JSON)
        let click = serde_json::from_str::<I3ClickEvent>(&line)?;

        // parse bar item index from the "instance" property
        let i = match click.instance.as_ref() {
            Some(inst) => match inst.parse::<usize>() {
                Ok(i) => i,
                Err(e) => cont!("Failed to parse click instance: {}, error: {}", inst, e),
            },
            None => cont!(
                "Received click event without instance, cannot route to item: {:?}",
                click
            ),
        };

        send_bar_event(&bar_tx[i], BarEvent::Click(click)).await?;
    }
}

async fn send_bar_event(tx: &mpsc::Sender<BarEvent>, ev: BarEvent) -> Result<(), Box<dyn Error>> {
    // if the channel fills up (the bar never reads click events), since this is a bounded channel
    // sending the event would block forever, so just drop the event
    if tx.capacity() == 0 {
        return Err("Could not send event to item, dropping event (channel is full)".into());
    }

    // send click event to its corresponding bar item
    if let Err(SendError(_)) = tx.send(ev).await {
        return Err("Could not send event to item, dropping event (receiver dropped)".into());
    }

    Ok(())
}
