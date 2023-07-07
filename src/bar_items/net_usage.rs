use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use strum::EnumIter;
use sysinfo::{NetworkExt, NetworksExt, SystemExt};
use tokio::time::Instant;

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::error::Result;
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::EnumCycle;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
enum UsageDisplay {
    // as bits: 1 Kbit == 8000 bits == 1000 B
    Bits,
    // as bytes: 1 KB == 1000 B
    #[default]
    Bytes,
    // as bibytes: 1 KiB == 1024 B
    Bibytes,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NetUsage {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    minimum: Option<ByteSize>,
    #[serde(default)]
    thresholds: Vec<ByteSize>,
    #[serde(default)]
    ignored_interfaces: Vec<String>,
    #[serde(default)]
    display: UsageDisplay,
    /// Currently only surfaced for testing.
    #[serde(default)]
    _always_assume_interval: bool,
}

impl NetUsage {
    fn get_color(&self, theme: &Theme, bytes: u64) -> Option<HexColor> {
        if self.thresholds.len() == 0 {
            return None;
        }

        let end = self
            .thresholds
            .first()
            .map(|b| b.as_u64())
            .unwrap_or(u64::MAX);

        if (0..=end).contains(&bytes) {
            return Some(theme.dim);
        }

        // NOTE: since we have 5 thresholds, and windows of 2, there will only be 4 windows
        // so we only need to map it to 4 colours here
        let threshold_colors = &[
            None,
            Some(theme.yellow),
            Some(theme.orange),
            Some(theme.red),
        ];
        for (idx, w) in self.thresholds.windows(2).enumerate() {
            if (w[0].as_u64()..w[1].as_u64()).contains(&bytes) {
                return threshold_colors[idx];
            }
        }

        // it was above any of the thresholds listed
        Some(theme.purple)
    }
}

fn format_bytes(bytes: u64, si: bool, as_bits: bool) -> String {
    let mut s = ByteSize(if as_bits { bytes * 8 } else { bytes }).to_string_as(si);
    if as_bits {
        s.pop();
        format!("{}bits", s)
    } else {
        s
    }
}

#[async_trait(?Send)]
impl BarItem for NetUsage {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let fg = |bytes: u64, theme: &Theme| {
            self.get_color(&theme, bytes)
                .map(|c| format!(r#" foreground="{}""#, c))
                .unwrap_or("".into())
        };

        let min = self.minimum.map_or(bytesize::KIB, |b| b.as_u64());
        let text = |bytes, display| {
            format!(
                "{:>8}",
                if bytes >= min {
                    match display {
                        UsageDisplay::Bits => format_bytes(bytes, false, true),
                        UsageDisplay::Bytes => format_bytes(bytes, false, false),
                        UsageDisplay::Bibytes => format_bytes(bytes, true, false),
                    }
                } else {
                    "-".into()
                }
            )
        };

        let mut display = EnumCycle::new_at(self.display)?;

        let div_as_u64 = |u, f| (u as f64 / f) as u64;
        let mut last_check = Instant::now();
        loop {
            let (down, up) = {
                let networks = ctx.state.sys.networks_mut();

                // NOTE: can call `networks.refresh()` instead of this to only update networks rather
                // than searching for new ones each time
                networks.refresh_networks_list();

                // this returns the number of bytes since the last refresh
                let (down, up) = networks.iter().fold((0, 0), |(d, u), (interface, net)| {
                    if self.ignored_interfaces.contains(&interface) {
                        (d, u)
                    } else {
                        (d + net.received(), u + net.transmitted())
                    }
                });

                // so we check how long it's been since the last refresh, and adjust accordingly
                let elapsed = last_check.elapsed().as_secs_f64();
                last_check = Instant::now();

                if self._always_assume_interval {
                    (down, up)
                } else {
                    (div_as_u64(down, elapsed), div_as_u64(up, elapsed))
                }
            };

            ctx.update_item(
                I3Item::new(format!(
                    "<span{}>{}↓</span> <span{}>{}↑</span>",
                    fg(down, &ctx.config.theme),
                    text(down, *display.current()),
                    fg(up, &ctx.config.theme),
                    text(up, *display.current())
                ))
                .markup(I3Markup::Pango),
            )
            .await?;

            // swap between bits and bytes on click
            if let Some(event) = ctx.wait_for_event(Some(self.interval)).await {
                if let BarEvent::Click(click) = event {
                    if click.button == I3Button::Left {
                        display.next();
                    }
                }
            }
        }
    }
}
