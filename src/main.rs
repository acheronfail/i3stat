mod i3;
mod item;

use std::error::Error;

use i3::*;
use item::*;
use sysinfo::{
    System,
    SystemExt,
};

use crate::item::{
    battery::Battery,
    cpu::Cpu,
    disk::Disk,
    dunst::Dunst,
    mem::Mem,
    net_usage::NetUsage,
    nic::Nic,
    script::Script,
    sensors::Sensors,
    time::Time,
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
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    println!("{}", json!(I3BarHeader::default()));
    println!("[");

    let mut sys = System::new_all();
    let mut bar = Bar(vec![
        // Box::new(Item::text("Hello")),
        // Box::new(Time::default()),
        // Box::new(Cpu::default()),
        // Box::new(NetUsage::default()),
        // Box::new(Nic::default()),
        // Box::new(Battery::default()),
        // Box::new(Mem::default()),
        // Box::new(Disk::default()),
        // Box::new(Dunst::default()),
        // Box::new(Sensors::default()),
        Box::new(Script::default()),
    ]);

    loop {
        // TODO: different update times per item
        // TODO: create context, which contains
        //      sysinfo::System
        //      dbus connection
        //      ... any other shared things ...
        bar.update(&mut sys);

        println!("{},", json!(bar));

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
