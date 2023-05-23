use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{Disk as SysDisk, DiskExt, SystemExt};

use crate::context::{BarItem, Context};
use crate::format::fraction;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Disk {
    #[serde(with = "crate::human_time")]
    interval: Duration,
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
            0..=10 => Some(theme.red),
            11..=20 => Some(theme.orange),
            21..=30 => Some(theme.yellow),
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
                let mut state = ctx.state.borrow_mut();
                state.sys.refresh_disks();
                state.sys.refresh_disks_list();
                state.sys.disks().iter().map(DiskStats::from_disk).collect()
            };
            let len = stats.len();
            if len > 0 {
                idx = idx % len;

                let disk = &stats[idx];
                let (full, short) = disk.format();
                let full = format!("{}{}", full, fraction(&ctx.theme, idx + 1, len));

                let mut item = I3Item::new(full).short_text(short).markup(I3Markup::Pango);

                if let Some(fg) = disk.get_color(&ctx.theme) {
                    item = item.color(fg);
                }

                ctx.update_item(item).await?;
            }

            // cycle through disks
            ctx.delay_with_event_handler(self.interval, |event| {
                Context::paginate(&event, len, &mut idx);
                async {}
            })
            .await;
        }
    }
}
