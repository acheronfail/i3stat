use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{Disk as SysDisk, Disks};

use crate::context::{BarItem, Context, StopAction};
use crate::error::Result;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::{expand_path, Paginator};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskAlias {
    path: PathBuf,
    name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Disk {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    #[serde(default)]
    mounts: HashSet<PathBuf>,
    #[serde(default)]
    aliases: Vec<DiskAlias>,
}

struct DiskStats {
    alias: Option<String>,
    mount_point: PathBuf,
    available_bytes: u64,
    total_bytes: u64,
}

impl DiskStats {
    fn new(disk: &SysDisk, alias: Option<String>) -> DiskStats {
        DiskStats {
            alias,
            mount_point: disk.mount_point().to_path_buf(),
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
        let name = self
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.mount_point.to_string_lossy().to_string());

        (
            format!(
                "󰋊 {} {}",
                name,
                ByteSize(self.available_bytes).to_string_as(true)
            ),
            name,
        )
    }
}

#[async_trait(?Send)]
impl BarItem for Disk {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let mut p = Paginator::new();
        let mut disks = Disks::new();
        loop {
            let stats: Vec<DiskStats> = {
                disks.refresh();
                disks.refresh_list();
                disks
                    .iter()
                    .filter(|d| {
                        if self.mounts.is_empty() {
                            true
                        } else {
                            self.mounts.contains(d.mount_point())
                        }
                    })
                    .map(|d| {
                        DiskStats::new(
                            d,
                            self.aliases
                                .iter()
                                .find(|a| {
                                    expand_path(&a.path).map_or(false, |p| p == d.mount_point())
                                })
                                .map(|a| a.name.clone()),
                        )
                    })
                    .collect()
            };
            let len = stats.len();
            if len > 0 {
                p.set_len(len)?;

                let disk = &stats[p.idx()];
                let theme = &ctx.config.theme;
                let (full, short) = disk.format(theme);
                let full = format!("{}{}", full, p.format(theme));

                let mut item = I3Item::new(full)
                    .short_text(short)
                    .markup(I3Markup::Pango)
                    .with_data("mount_point", disk.mount_point.to_string_lossy().into());

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
