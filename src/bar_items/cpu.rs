use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use sysinfo::{CpuExt, CpuRefreshKind, SystemExt};

use crate::context::{BarItem, Context};
use crate::i3::I3Item;

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
    pub fn get_full_text(&self, pct: f32) -> String {
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
            "{:0pad$.precision$}%",
            pct,
            precision = self.precision,
            pad = pad
        )
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

            ctx.update_item(I3Item::new(self.get_full_text(pct)).name("cpu"))
                .await
                .unwrap();

            ctx.delay_with_click_handler(self.interval, |_| {
                todo!("open CPU monitor if it's not already open");
            })
            .await;
        }
    }
}
