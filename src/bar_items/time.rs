use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use chrono::prelude::*;

use crate::context::{BarItem, Context};
use crate::exec::exec;
use crate::i3::{I3Button, I3Item};
use crate::BarEvent;

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
            short_format: "%H:%M".into(),
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Time {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let now = Local::now();
            let item = I3Item::new(now.format(&self.full_format).to_string())
                .name("time")
                .short_text(now.format(&self.short_format).to_string());

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
