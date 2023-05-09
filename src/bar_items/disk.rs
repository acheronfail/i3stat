use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use sysinfo::{Disk as SysDisk, DiskExt, SystemExt};

use crate::context::{BarItem, Context};
use crate::i3::{I3Button, I3Item};
use crate::theme::Theme;
use crate::BarEvent;

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

struct DiskStats {
    mount_point: String,
    available_bytes: u64,
    total_bytes: u64,
}

impl DiskStats {
    fn from_disk(disk: &SysDisk) -> DiskStats {
        DiskStats {
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            available_bytes: disk.available_space(),
            total_bytes: disk.total_space(),
        }
    }

    fn get_color(&self, theme: &Theme) -> Option<HexColor> {
        let pct = (self.available_bytes as f64 / self.total_bytes as f64) * 100.0;
        match pct as u32 {
            0..=10 => Some(theme.error),
            11..=20 => Some(theme.danger),
            21..=30 => Some(theme.warning),
            _ => None,
        }
    }

    fn format(&self) -> (String, String) {
        (
            format!(
                "󰋊 {} {}",
                self.mount_point,
                ByteSize(self.available_bytes).to_string_as(true)
            ),
            format!("{}", self.mount_point),
        )
    }
}

#[async_trait(?Send)]
impl BarItem for Disk {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        let mut idx = 0;
        loop {
            let stats: Vec<DiskStats> = {
                let mut state = ctx.state.lock().unwrap();
                state.sys.refresh_disks();
                state.sys.disks().iter().map(DiskStats::from_disk).collect()
            };
            let len = stats.len();
            idx = idx % len;

            let disk = &stats[idx];
            let (full, short) = disk.format();
            let mut item = I3Item::new(full).short_text(short).name("disk");
            if let Some(fg) = disk.get_color(&ctx.theme) {
                item = item.color(fg);
            }
            ctx.update_item(item).await?;

            // cycle through disks
            ctx.delay_with_event_handler(self.interval, |event| {
                if let BarEvent::Click(click) = event {
                    match click.button {
                        I3Button::Left | I3Button::ScrollUp => idx += 1,
                        I3Button::Right | I3Button::ScrollDown => {
                            if idx == 0 {
                                idx = len - 1
                            } else {
                                idx -= 1
                            }
                        }
                        _ => {}
                    }
                }

                async {}
            })
            .await;
        }
    }
}
