use crate::error::Result;
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{CpuExt, CpuRefreshKind, SystemExt};

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::exec;
use crate::util::format::{float, FloatFormat};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Cpu {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    #[serde(flatten)]
    float_fmt: FloatFormat,
}

impl Cpu {
    fn get_full_text(&self, _: &Theme, pct: f32) -> String {
        format!(" {}%", float(pct, &self.float_fmt))
    }

    fn get_color(&self, theme: &Theme, pct: f32) -> Option<HexColor> {
        match pct as u64 {
            80..=100 => Some(theme.red),
            60..=79 => Some(theme.orange),
            40..=59 => Some(theme.yellow),
            _ => None,
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Cpu {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        loop {
            let pct = {
                // refresh cpu usage
                ctx.state
                    .sys
                    .refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());
                // fetch cpu usage since we last refreshed
                ctx.state.sys.global_cpu_info().cpu_usage()
            };

            let theme = &ctx.config.theme;
            let mut item = I3Item::new(self.get_full_text(theme, pct)).markup(I3Markup::Pango);
            if let Some(fg) = self.get_color(theme, pct) {
                item = item.color(fg);
            }

            ctx.update_item(item).await?;
            ctx.delay_with_event_handler(self.interval, |event| async move {
                if let BarEvent::Click(_) = event {
                    exec("systemmonitor").await;
                }
            })
            .await;
        }
    }
}
