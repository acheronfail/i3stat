use bytesize::ByteSize;
use sysinfo::{
    NetworkExt,
    NetworksExt,
    System,
    SystemExt,
};

use super::{
    Item,
    ToItem,
};

pub struct NetUsage {
    bytes_down: u64,
    bytes_up: u64,
}

impl Default for NetUsage {
    fn default() -> Self {
        NetUsage {
            bytes_down: 0,
            bytes_up: 0,
        }
    }
}

impl ToItem for NetUsage {
    fn to_item(&self) -> Item {
        Item::text(format!(
            "↓{} ↑{}",
            ByteSize(self.bytes_down).to_string_as(true),
            ByteSize(self.bytes_up).to_string_as(true)
        ))
    }

    fn update(&mut self, sys: &System) {
        let (down, up) = sys.networks().iter().fold((0, 0), |(d, u), (_, net)| {
            (d + net.received(), u + net.transmitted())
        });

        self.bytes_down = down;
        self.bytes_up = up;
    }
}
