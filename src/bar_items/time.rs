use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use chrono::prelude::*;

use crate::context::{BarItem, Context};
use crate::i3::{I3Button, I3Item};

pub struct Time {
    interval: Duration,
    full_format: String,
    short_format: String,
}

impl Default for Time {
    fn default() -> Self {
        Time {
            interval: Duration::from_secs(1),
            full_format: "%Y-%m-%d %H:%M:%S".into(),
            short_format: "%m/%d %H:%M".into(),
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Time {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let now = Local::now();
            let item = I3Item::new(now.format(&self.full_format).to_string())
                .short_text(now.format(&self.short_format).to_string());

            ctx.update_item(item).await?;

            // Wait for "refresh" time, OR if a click comes through, then update
            ctx.delay_with_click_handler(self.interval, |click| match click.button {
                I3Button::Left => todo!("open gsimplecal/etc"),
                _ => {}
            })
            .await;
        }
    }
}
