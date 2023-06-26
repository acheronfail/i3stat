use crate::error::Result;
use std::time::Duration;

use async_trait::async_trait;
use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::util::exec;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Time {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    format_long: String,
    format_short: String,
}

#[async_trait(?Send)]
impl BarItem for Time {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        loop {
            let now = Local::now();
            let item = I3Item::new(format!("󰥔 {}", now.format(&self.format_long)))
                .short_text(now.format(&self.format_short).to_string())
                .markup(I3Markup::Pango);

            ctx.update_item(item).await?;

            ctx.delay_with_event_handler(self.interval, |event| async move {
                match event {
                    BarEvent::Click(click) => match click.button {
                        I3Button::Left => exec("gsimplecal").await,
                        _ => {}
                    },
                    _ => {}
                }
            })
            .await;
        }
    }
}
