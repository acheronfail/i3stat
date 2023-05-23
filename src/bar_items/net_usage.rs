use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{NetworkExt, NetworksExt, SystemExt};
use tokio::time::Instant;

use crate::context::{BarEvent, BarItem, Context};
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NetUsage {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    minimum: Option<ByteSize>,
    thresholds: Vec<ByteSize>,
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
            Some(theme.warning),
            Some(theme.danger),
            Some(theme.error),
        ];
        for (idx, w) in self.thresholds.windows(2).enumerate() {
            if (w[0].as_u64()..w[1].as_u64()).contains(&bytes) {
                return threshold_colors[idx];
            }
        }

        // it was above any of the thresholds listed
        Some(theme.special)
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
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        let theme = ctx.theme.clone();
        let fg = |bytes| {
            self.get_color(&theme, bytes)
                .map(|c| format!(r#" foreground="{}""#, c))
                .unwrap_or("".into())
        };

        let min = self.minimum.map_or(bytesize::KIB, |b| b.as_u64());
        let text = |bytes, as_bits| {
            format!(
                "{:>8}",
                if bytes > min {
                    format_bytes(bytes, false, as_bits)
                } else {
                    "-".into()
                }
            )
        };

        let div_as_u64 = |u, f| (u as f64 / f) as u64;
        let mut last_check = Instant::now();
        let mut as_bits = false;
        loop {
            let (down, up) = {
                let mut state = ctx.state.borrow_mut();
                let networks = state.sys.networks_mut();

                // NOTE: can call `networks.refresh()` instead of this to only update networks rather
                // than searching for new ones each time
                networks.refresh_networks_list();

                // this returns the number of bytes since the last refresh
                let (down, up) = networks.iter().fold((0, 0), |(d, u), (_, net)| {
                    (d + net.received(), u + net.transmitted())
                });

                // so we check how long it's been since the last refresh, and adjust accordingly
                let elapsed = last_check.elapsed().as_secs_f64();
                last_check = Instant::now();

                (div_as_u64(down, elapsed), div_as_u64(up, elapsed))
            };

            ctx.update_item(
                I3Item::new(format!(
                    "<span{}>{}↓</span> <span{}>{}↑</span>",
                    fg(down),
                    text(down, as_bits),
                    fg(up),
                    text(up, as_bits)
                ))
                .markup(I3Markup::Pango),
            )
            .await?;

            // swap between bits and bytes on click
            if let Some(event) = ctx.wait_for_event(Some(self.interval)).await {
                if let BarEvent::Click(click) = event {
                    if click.button == I3Button::Left {
                        as_bits = !as_bits;
                    }
                }
            }
        }
    }
}
