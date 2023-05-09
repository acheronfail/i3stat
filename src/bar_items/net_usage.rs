use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use sysinfo::{NetworkExt, NetworksExt, SystemExt};
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

pub struct NetUsage {
    interval: Duration,
}

impl Default for NetUsage {
    fn default() -> Self {
        NetUsage {
            interval: Duration::from_secs(1),
        }
    }
}

impl NetUsage {
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
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        let fg = |bytes| {
            Self::get_color(&ctx.theme, bytes)
                .map(|c| format!(r#" foreground="{}""#, c))
                .unwrap_or("".into())
        };

        let text = |bytes| {
            if bytes > bytesize::KIB {
                // TODO: can we get two decimal places?
                ByteSize(bytes).to_string_as(true)
            } else {
                "0".into()
            }
        };

        // TODO: click to cycle between bits and bytes
        loop {
            let (down, up) = {
                let mut state = ctx.state.lock().unwrap();
                state.sys.refresh_networks();
                state
                    .sys
                    .networks()
                    .iter()
                    .fold((0, 0), |(d, u), (_, net)| {
                        (d + net.received(), u + net.transmitted())
                    })
            };

            ctx.update_item(
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

            sleep(self.interval).await;
        }
    }
}
