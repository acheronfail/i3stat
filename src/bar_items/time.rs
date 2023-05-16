use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarEvent, BarItem, Context};
use crate::exec::exec;
use crate::i3::{I3Button, I3Item};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Time {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    format_long: String,
    format_short: String,
}

#[async_trait(?Send)]
impl BarItem for Time {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let now = Local::now();
            let item = I3Item::new(now.format(&self.format_long).to_string())
                .name("time")
                .short_text(now.format(&self.format_short).to_string());

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
