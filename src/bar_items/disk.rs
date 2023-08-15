use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{Disk as SysDisk, DiskExt, SystemExt};

use crate::context::{BarItem, Context, StopAction};
use crate::error::Result;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::Paginator;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Disk {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    #[serde(default)]
    mounts: HashSet<PathBuf>,
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

    fn format(&self, _: &Theme) -> (String, String) {
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
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let mut p = Paginator::new();
        loop {
            let stats: Vec<DiskStats> = {
                ctx.state.sys.refresh_disks();
                ctx.state.sys.refresh_disks_list();
                ctx.state
                    .sys
                    .disks()
                    .iter()
                    .filter(|d| {
                        if self.mounts.is_empty() {
                            true
                        } else {
                            self.mounts.contains(d.mount_point())
                        }
                    })
                    .map(DiskStats::from_disk)
                    .collect()
            };
            let len = stats.len();
            if len > 0 {
                p.set_len(len)?;

                let disk = &stats[p.idx()];
                let theme = &ctx.config.theme;
                let (full, short) = disk.format(theme);
                let full = format!("{}{}", full, p.format(theme));

                let mut item = I3Item::new(full).short_text(short).markup(I3Markup::Pango);

                if let Some(fg) = disk.get_color(theme) {
                    item = item.color(fg);
                }

                ctx.update_item(item).await?;
            } else {
                ctx.update_item(I3Item::empty()).await?;
            }

            // cycle through disks
            ctx.delay_with_event_handler(self.interval, |event| {
                p.update(&event);
                async {}
            })
            .await;
        }
    }
}
