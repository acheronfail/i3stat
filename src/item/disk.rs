use bytesize::ByteSize;
use sysinfo::{
    DiskExt,
    System,
    SystemExt,
};

use super::{
    Item,
    ToItem,
};

pub struct Disk {
    inner: Vec<(String, u64)>,
}

impl Default for Disk {
    fn default() -> Self {
        Disk { inner: vec![] }
    }
}

impl ToItem for Disk {
    fn to_item(&self) -> Item {
        Item::new(
            self.inner
                .iter()
                .map(|(mount_point, available_bytes)| {
                    format!(
                        "{}: {}",
                        mount_point,
                        ByteSize(*available_bytes).to_string_as(true)
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    fn update(&mut self, sys: &mut System) {
        // sys.refresh_disks();
        sys.refresh_disks_list();

        self.inner = sys
            .disks()
            .iter()
            .map(|d| {
                (
                    d.mount_point().to_string_lossy().into_owned(),
                    d.available_space(),
                )
            })
            .collect();
    }
}
