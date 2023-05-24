use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use tokio::process::Command;

use crate::context::{BarItem, Context};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Krb {
    #[serde(with = "crate::human_time::option")]
    interval: Option<Duration>,
}

impl Krb {
    async fn get_state(&self) -> Result<bool, Box<dyn Error>> {
        let output = Command::new("klist").arg("-s").output().await?;
        Ok(output.status.success())
    }

    async fn item(&self, theme: &Theme) -> Result<I3Item, Box<dyn Error>> {
        Ok(I3Item::new("󱕵")
            .markup(I3Markup::Pango)
            .color(if self.get_state().await? {
                theme.fg
            } else {
                theme.dim
            }))
    }
}

#[async_trait(?Send)]
impl BarItem for Krb {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            ctx.update_item(self.item(&ctx.theme).await?).await?;

            ctx.wait_for_event(self.interval).await;
        }
    }
}
