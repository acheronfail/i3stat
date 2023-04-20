mod context;
mod i3;
mod item;

use std::error::Error;

use i3::*;
use tokio::io::stdin;
use tokio::io::{
    AsyncBufReadExt,
    BufReader,
};
use tokio::sync::mpsc;

use crate::{
    context::Context,
    item::{
        script::Script,
        time::Time,
        Item,
    },
};

macro_rules! json {
    ($input:expr) => {
        serde_json::to_string(&$input).unwrap()
    };
}

// TODO: central place for storing formatting options? (precision, GB vs G, padding, etc)
// TODO: use an event loop to manage timers and refreshes for items, as well as stop blocking things
// (like dbus) from blocking everything else
//  - need a way for items to trigger updates, etc
// TODO: config file? how to setup blocks?
// TODO: tokio
//  event loop is: IPC events from i3 (clicks, signals, etc)
//  before event loop starts, need to spawn
//      blocks will likely have a `loop {}` in them for their infinite updates
//      should these be `spawn_blocking`?
//      should these be `thread::spawn`? (how to share context?)
//  TODO: I want click updates to come immediately, not have to wait for main thread - can i do this with tokio?
//      it's multi-thread executor by default, so not a huge prob
//      but also, can use `spawn_blocking` and other things to mitigate
//      and to fully mitigate, can just spawn all blocks in separate threads
// TODO: decision 1 - just use tokio for everything, and if things are slow, then spawn different threads

pub struct Sender {
    inner: tokio::sync::mpsc::Sender<(Item, usize)>,
    index: usize,
}

impl Sender {
    pub fn new(tx: tokio::sync::mpsc::Sender<(Item, usize)>, index: usize) -> Sender {
        Sender { inner: tx, index }
    }

    pub async fn send(
        &self,
        item: Item,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<(Item, usize)>> {
        self.inner.send((item, self.index)).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO: experiment with signal and mutex (deadlocks)

    println!("{}", json!(I3BarHeader::default()));
    println!("[");

    let items: Vec<Box<dyn BarItem>> = vec![
        Box::new(Item::new("text")),
        Box::new(Time::default()),
        // Box::new(Cpu::default()),
        // Box::new(NetUsage::default()),
        // Box::new(Nic::default()),
        // Box::new(Battery::default()),
        // Box::new(Mem::default()),
        // Box::new(Disk::default()),
        // Box::new(Dunst::default()),
        // Box::new(Sensors::default()),
        Box::new(Script::default()),
    ];
    let bar_item_count = items.len();

    // shared context
    let ctx = Context::new();
    let (tx, mut rx) = mpsc::channel(1);
    for (i, bar_item) in items.into_iter().enumerate() {
        let ctx = ctx.clone();
        let sender = Sender::new(tx.clone(), i);
        tokio::spawn(async move {
            let fut = bar_item.start(ctx, sender);
            fut.await;
        });
    }

    let bar_printer = tokio::spawn(async move {
        let mut bar: Vec<Item> = vec![Item::empty(); bar_item_count];

        // TODO: should this be in a thread of its own? what about incoming IPC events?
        while let Some((item, i)) = rx.recv().await {
            bar[i] = item;
            // TODO: should I print the entire bar on any update of a child?
            //      does the bar protocol allow updates to specific bars?
            println!("{},", json!(bar));
        }
    });

    let s = BufReader::new(stdin());
    let mut lines = s.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line == "[" {
            continue;
        }
        let click = serde_json::from_str::<I3ClickEvent>(&line).unwrap();
        // TODO: send the click event to the right block
        dbg!(click);
    }

    // TODO: rather than this, open event loop for signals and click events
    bar_printer.await.unwrap();
    futures::future::pending::<()>().await;

    Ok(())

    // loop {
    //     // TODO: different update times per item
    //     // TODO: create context, which contains
    //     //      sysinfo::System
    //     //      dbus connection
    //     //      ... any other shared things ...
    //     // bar.update(&mut sys);

    //     println!("{},", json!(bar));

    //     std::thread::sleep(std::time::Duration::from_secs(1));
    // }
}
