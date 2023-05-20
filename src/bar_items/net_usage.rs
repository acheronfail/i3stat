use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{NetworkExt, NetworksExt, SystemExt};
use tokio::time::{sleep, Instant};

use crate::context::{BarItem, Context};
use crate::i3::{I3Item, I3Markup};
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
            return Some(theme.dark4);
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

#[async_trait(?Send)]
impl BarItem for NetUsage {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        // this item doesn't receive any input, so close the receiver
        ctx.raw_event_rx().close();

        let fg = |bytes| {
            self.get_color(&ctx.theme, bytes)
                .map(|c| format!(r#" foreground="{}""#, c))
                .unwrap_or("".into())
        };

        let min = self.minimum.map_or(bytesize::KIB, |b| b.as_u64());
        let text = |bytes| {
            if bytes > min {
                ByteSize(bytes).to_string_as(true)
            } else {
                "-".into()
            }
        };

        let div_as_u64 = |u, f| (u as f64 / f) as u64;
        let mut last_check = Instant::now();
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
                // TODO: click to cycle between bits and bytes
                // https://github.com/hyunsik/bytesize/issues/30
                I3Item::new(format!(
                    "<span{}>{}↓</span> <span{}>{}↑</span>",
                    fg(down),
                    text(down),
                    fg(up),
                    text(up)
                ))
                .markup(I3Markup::Pango),
            )
            .await?;

            // this item sleeps rather than waiting for input, since that would affect the calculation interval
            sleep(self.interval).await;
        }
    }
}
