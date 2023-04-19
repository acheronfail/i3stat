use bytesize::ByteSize;
use sysinfo::{
    System,
    SystemExt,
};

use super::{
    Item,
    ToItem,
};

pub struct Mem {
    available: u64,
    used: u64,
    total: u64,
}

impl Default for Mem {
    fn default() -> Self {
        Mem {
            available: 0,
            used: 0,
            total: 0,
        }
    }
}

impl ToItem for Mem {
    fn to_item(&self) -> Item {
        Item::new(format!("{}", ByteSize(self.available).to_string_as(false)))
    }

    fn update(&mut self, sys: &mut System) {
        sys.refresh_memory();

        self.available = sys.available_memory();
        self.used = sys.used_memory();
        self.total = sys.total_memory();
    }
}
