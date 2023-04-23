use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use sysinfo::{DiskExt, SystemExt};
use tokio::time::sleep;

use crate::{BarItem, Item};
use crate::context::Context;

pub struct Disk {
    interval: Duration,
}

impl Default for Disk {
    fn default() -> Self {
        Disk {
            interval: Duration::from_secs(120),
        }
    }
}

#[async_trait]
impl BarItem for Disk {
    async fn start(&mut self, ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let stats: Vec<(String, u64)> = {
                let mut state = ctx.state.lock().unwrap();
                // TODO: only refresh the disk we want, not all of them
                state.sys.refresh_disks();
                state
                    .sys
                    .disks()
                    .iter()
                    .map(|d| {
                        (
                            d.mount_point().to_string_lossy().into_owned(),
                            d.available_space(),
                        )
                    })
                    .collect()
            };

            ctx.update_item(Item::new(
                stats
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
            ))
            .await?;

            sleep(self.interval).await;
        }
    }
}
