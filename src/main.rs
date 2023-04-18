mod i3;
mod item;

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
    mem::Mem,
    net_usage::NetUsage,
    nic::Nic,
    time::Time,
};

macro_rules! json {
    ($input:expr) => {
        serde_json::to_string(&$input).unwrap()
    };
}

fn main() {
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
        Box::new(Disk::default()),
        // TODO: temperature
        // TODO: dunst
        // TODO: scripts (amber price info, caffeinate)
    ]);
    loop {
        // TODO: different update times per item
        bar.update(&mut sys);

        println!("{},", json!(bar));

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
