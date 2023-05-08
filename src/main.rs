#![feature(exclusive_range_pattern)]

mod context;
mod exec;
pub mod i3;
mod theme;
mod bar_items {
    automod::dir!(pub "src/bar_items");
    // TODO: https://github.com/dtolnay/automod/issues/15
    pub mod dunst;
    pub mod pulse;
}

use std::error::Error;

use tokio::io::{stdin, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::bar_items::battery::Battery;
use crate::bar_items::cpu::Cpu;
use crate::bar_items::disk::Disk;
use crate::bar_items::dunst::Dunst;
use crate::bar_items::mem::Mem;
use crate::bar_items::net_usage::NetUsage;
use crate::bar_items::nic::Nic;
use crate::bar_items::pulse::Pulse;
use crate::bar_items::script::Script;
use crate::bar_items::sensors::Sensors;
use crate::bar_items::time::Time;
use crate::context::{Context, SharedState};
use crate::i3::click::I3ClickEvent;
use crate::i3::header::I3BarHeader;

macro_rules! json {
    ($input:expr) => {
        serde_json::to_string(&$input).unwrap()
    };
}

// TODO: central place for storing formatting options? (precision, GB vs G, padding, etc)
// TODO: config file? how to setup blocks?

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    tokio::task::LocalSet::new().block_on(&runtime, async move { async_main().await })
}

async fn async_main() -> Result<(), Box<dyn Error>> {
    println!("{}", json!(I3BarHeader::default()));
    println!("[");

    let items: Vec<Box<dyn context::BarItem>> = vec![
        Box::new(NetUsage::default()),
        Box::new(Nic::default()),
        Box::new(Disk::default()),
        Box::new(Cpu::default()),
        Box::new(Sensors::default()),
        Box::new(Mem::default()),
        Box::new(Pulse::default()),
        Box::new(Battery::default()),
        Box::new(Time::default()),
        Box::new(Script::default()),
        Box::new(Dunst::default()),
    ];
    let bar_item_count = items.len();

    // shared context
    let state = SharedState::new();

    // state for the bar (moved to bar_printer)
    let mut bar: Vec<i3::I3Item> = vec![i3::I3Item::empty(); bar_item_count];
    let mut bar_tx: Vec<mpsc::Sender<I3ClickEvent>> = Vec::with_capacity(bar_item_count);

    // for each BarItem, spawn a new task to manage it
    let (item_tx, mut item_rx) = mpsc::channel(bar_item_count);
    for (i, bar_item) in items.into_iter().enumerate() {
        let (click_tx, click_rx) = mpsc::channel(32);
        bar_tx.push(click_tx);
        let ctx = Context::new(state.clone(), item_tx.clone(), click_rx, i);
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

    // IPC click event loop from i3
    let s = BufReader::new(stdin());
    let mut lines = s.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        // skip opening array as part of the protocol
        if line == "[" {
            continue;
        }

        macro_rules! cont {
            ($($arg:tt)*) => {{
                eprintln!($($arg)*);
                continue;
            }};
        }

        // parse click event (single line JSON)
        let click = serde_json::from_str::<I3ClickEvent>(&line).unwrap();

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

        // if the channel fills up (the bar never reads click events), since this is a bounded channel
        // sending the event would block forever, so just drop the event
        let tx = &bar_tx[i];
        if tx.capacity() == 0 {
            cont!("Could not send click event to block, dropping event (channel is full)");
        }

        // send click event to its corresponding bar item
        if let Err(SendError(click)) = tx.send(click).await {
            cont!(
                "Received click event for block that is no longer receving: {:?}",
                click
            );
        }
    }

    eprintln!("STDIN was closed, exiting");
    Ok(())
}
