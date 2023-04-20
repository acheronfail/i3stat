use std::time::Duration;

use async_trait::async_trait;
use chrono::prelude::*;
use tokio::time;

use super::Item;
use crate::{
    context::Ctx,
    BarItem,
    Sender,
};

pub struct Time {
    full_format: String,
    short_format: String,
}

impl Default for Time {
    fn default() -> Self {
        Time {
            full_format: "%Y-%m-%d %H:%M:%S".into(),
            short_format: "%m/%d %H:%M".into(),
        }
    }
}

#[async_trait]
impl BarItem for Time {
    async fn start(&self, _: Ctx, tx: Sender) {
        loop {
            let now = Local::now();
            let item = Item::new(now.format(&self.full_format).to_string())
                .short_text(now.format(&self.short_format).to_string());

            tx.send(item).await.unwrap();
            time::sleep(Duration::from_secs(1)).await;
        }
    }
}
