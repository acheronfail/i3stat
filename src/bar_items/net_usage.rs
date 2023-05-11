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

#[derive(Debug, Serialize, Deserialize)]
pub struct NetUsage {
    #[serde(with = "humantime_serde")]
    interval: Duration,
}

impl NetUsage {
    // TODO: make these configurable
    const THRESHOLD_1: u64 = bytesize::KIB;
    const THRESHOLD_2: u64 = bytesize::MIB;
    const THRESHOLD_3: u64 = bytesize::MIB * 10;
    const THRESHOLD_4: u64 = bytesize::MIB * 25;
    const THRESHOLD_5: u64 = bytesize::MIB * 100;

    fn get_color(theme: &Theme, bytes: u64) -> Option<HexColor> {
        match bytes {
            0..Self::THRESHOLD_1 => Some(theme.dark4),
            Self::THRESHOLD_1..Self::THRESHOLD_2 => None,
            Self::THRESHOLD_2..Self::THRESHOLD_3 => Some(theme.warning),
            Self::THRESHOLD_3..Self::THRESHOLD_4 => Some(theme.danger),
            Self::THRESHOLD_4..Self::THRESHOLD_5 => Some(theme.error),
            Self::THRESHOLD_5..u64::MAX => Some(theme.special),
            _ => None,
        }
    }
}

#[async_trait(?Send)]
impl BarItem for NetUsage {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        // this item doesn't receive any input, so close the receiver
        ctx.raw_event_rx().close();

        let fg = |bytes| {
            Self::get_color(&ctx.theme, bytes)
                .map(|c| format!(r#" foreground="{}""#, c))
                .unwrap_or("".into())
        };

        let text = |bytes| {
            if bytes > bytesize::KIB {
                ByteSize(bytes).to_string_as(true)
            } else {
                "0".into()
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
                .name("net_usage")
                .markup(I3Markup::Pango),
            )
            .await?;

            // this item sleeps rather than waiting for input, since that would affect the calculation interval
            sleep(self.interval).await;
        }
    }
}
