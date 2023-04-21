use std::time::Duration;

use async_trait::async_trait;
use chrono::prelude::*;

use super::Item;
use crate::context::Context;
use crate::BarItem;

pub struct Time {
    count: usize,

    full_format: String,
    short_format: String,
}

impl Default for Time {
    fn default() -> Self {
        Time {
            count: 0,

            full_format: "%Y-%m-%d %H:%M:%S".into(),
            short_format: "%m/%d %H:%M".into(),
        }
    }
}

#[async_trait]
impl BarItem for Time {
    async fn start(&mut self, mut ctx: Context) {
        loop {
            let now = Local::now();
            let t = now.format(&self.full_format);
            let item = Item::new(format!("{} ({})", t, self.count))
                .short_text(now.format(&self.short_format).to_string());

            ctx.update_item(item).await.unwrap();

            // Wait for "refresh" time, OR if a click comes through, then update
            ctx.delay_with_click_handler(Duration::from_secs(1), |_| self.count += 1)
                .await;
        }
    }
}
