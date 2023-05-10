use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::SystemExt;
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize)]
pub struct Mem {
    #[serde(with = "humantime_serde")]
    interval: Duration,
}

impl Mem {
    fn get_color(theme: &Theme, available: u64, total: u64) -> Option<HexColor> {
        match (available as f64 / total as f64) as u64 {
            80..=100 => Some(theme.error),
            60..80 => Some(theme.danger),
            40..60 => Some(theme.warning),
            _ => None,
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Mem {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        // TODO: click to toggle between bytes and %
        let mut total = None;
        loop {
            let (available, total) = {
                let mut state = ctx.state.borrow_mut();
                state.sys.refresh_memory();
                (
                    state.sys.available_memory(),
                    *total.get_or_insert_with(|| state.sys.total_memory()),
                )
            };

            let s = ByteSize(available).to_string_as(false);
            let mut item = I3Item::new(format!(" {}", s)).name("mem");
            if let Some(fg) = Self::get_color(&ctx.theme, available, total) {
                item = item.color(fg);
            }

            ctx.update_item(item).await?;
            sleep(self.interval).await;
        }
    }
}
