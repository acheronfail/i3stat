use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use sysinfo::{CpuExt, CpuRefreshKind, SystemExt};

use crate::context::{BarItem, Context};
use crate::exec::exec;
use crate::i3::I3Item;
use crate::theme::Theme;

pub struct Cpu {
    precision: usize,
    zero_pad: bool,
    interval: Duration,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu {
            precision: 0,
            zero_pad: true,
            interval: Duration::from_secs(2),
        }
    }
}

impl Cpu {
    fn get_full_text(&self, pct: f32) -> String {
        let pad = if !self.zero_pad {
            0
        } else if self.precision > 0 {
            // two digits + decimal separator + precision
            self.precision + 3
        } else {
            // two digits only
            2
        };

        format!(
            "  {:0pad$.precision$}%",
            pct,
            precision = self.precision,
            pad = pad
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
