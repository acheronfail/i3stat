#![feature(result_option_inspect)]

mod context;
mod i3;
mod item;

use std::error::Error;

use i3::*;
use tokio::io::{stdin, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::context::{BarItem, Context, SharedState};
use crate::item::battery::Battery;
use crate::item::cpu::Cpu;
use crate::item::net_usage::NetUsage;
use crate::item::script::Script;
use crate::item::time::Time;
use crate::item::Item;

macro_rules! json {
    ($input:expr) => {
        serde_json::to_string(&$input).unwrap()
    };
}

// TODO: experiment with signals and mutex (deadlocks)
// TODO: central place for storing formatting options? (precision, GB vs G, padding, etc)
// TODO: use an event loop to manage timers and refreshes for items, as well as stop blocking things
// (like dbus) from blocking everything else
//  - need a way for items to trigger updates, etc
// TODO: config file? how to setup blocks?
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", json!(I3BarHeader::default()));
    println!("[");

    let items: Vec<Box<dyn BarItem>> = vec![
        Box::new(Item::new("text")),
        Box::new(Time::default()),
        Box::new(Cpu::default()),
        Box::new(NetUsage::default()),
        // Box::new(Nic::default()),
        Box::new(Battery::default()),
        // Box::new(Mem::default()),
        // Box::new(Disk::default()),
        // Box::new(Dunst::default()),
        // Box::new(Sensors::default()),
        Box::new(Script::default()),
        // TODO: pasource pasink
    ];
    let bar_item_count = items.len();

    // shared context
    let state = SharedState::new();

    // state for the bar (moved to bar_printer)
    let mut bar: Vec<Item> = vec![Item::empty(); bar_item_count];
    let mut bar_rx: Vec<mpsc::Sender<I3ClickEvent>> = Vec::with_capacity(bar_item_count);

    // for each BarItem, spawn a new task to manage it
    let (tx, mut rx) = mpsc::channel(1);
    for (i, mut bar_item) in items.into_iter().enumerate() {
        let (item_tx, item_rx) = mpsc::channel(32);
        bar_rx.push(item_tx);
        let ctx = Context::new(state.clone(), tx.clone(), item_rx, i);
        tokio::spawn(async move {
            let fut = bar_item.start(ctx);
            fut.await;
        });
    }

    // task to manage updating the bar and printing it as JSON
    // TODO: buffer these and only print a single line within a threshold (no point in super quick updates)
    tokio::spawn(async move {
        while let Some((item, i)) = rx.recv().await {
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

        // send click event to its corresponding bar item
        if let Err(SendError(click)) = bar_rx[i].send(click).await {
            cont!(
                "Received click event for block that is no longer receving: {:?}",
                click
            );
        }
    }

    eprintln!("STDIN was closed, exiting");
    Ok(())
}
