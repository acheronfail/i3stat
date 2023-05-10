use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{CpuExt, CpuRefreshKind, SystemExt};

use crate::context::{BarItem, Context};
use crate::exec::exec;
use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize)]
pub struct Cpu {
    #[serde(with = "humantime_serde")]
    interval: Duration,
    #[serde(default)]
    precision: usize,
    pad: Option<char>,
    pad_count: Option<usize>,
}

impl Cpu {
    fn get_full_text(&self, pct: f32) -> String {
        let pad_count = self.pad_count.unwrap_or_else(|| {
            if self.precision > 0 {
                // three digits (e.g., 100%) + decimal separator + precision
                3 + 1 + self.precision
            } else {
                // three digits (e.g., 100%) only
                3
            }
        });

        let padding = self
            .pad
            .map(|c| {
                let s = c.to_string();
                let len = (pct.log10() + 1.).floor() as usize;
                if len >= pad_count {
                    "".into()
                } else {
                    s.repeat(pad_count - len)
                }
            })
            .unwrap_or("".into());

        format!(
            "  {}{:.precision$}%",
            padding,
            pct,
            precision = self.precision,
        )
    }

    fn get_color(&self, theme: &Theme, pct: f32) -> Option<HexColor> {
        match pct as u64 {
            80..=100 => Some(theme.error),
            60..80 => Some(theme.danger),
            40..60 => Some(theme.warning),
            _ => None,
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Cpu {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let pct = {
                let mut state = ctx.state.lock().unwrap();
                // refresh cpu usage
                state
                    .sys
                    .refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());
                // fetch cpu usage since we last refreshed
                state.sys.global_cpu_info().cpu_usage()
            };

            let mut item = I3Item::new(self.get_full_text(pct)).name("cpu");
            if let Some(fg) = self.get_color(&ctx.theme, pct) {
                item = item.color(fg);
            }

            ctx.update_item(item).await?;
            ctx.delay_with_event_handler(self.interval, |_| async {
                exec("systemmonitor").await;
            })
            .await;
        }
    }
}
