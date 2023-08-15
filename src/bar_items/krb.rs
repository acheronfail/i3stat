use std::time::Duration;

use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use tokio::process::Command;

use crate::context::{BarItem, Context, StopAction};
use crate::error::Result;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::filter::InterfaceFilter;
use crate::util::net_subscribe;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Krb {
    #[serde(default, with = "crate::human_time::option")]
    interval: Option<Duration>,
    #[serde(default)]
    only_on: Vec<InterfaceFilter>,
}

impl Krb {
    async fn get_state(&self) -> Result<bool> {
        let output = Command::new("klist").arg("-s").output().await?;
        Ok(output.status.success())
    }

    async fn item(&self, theme: &Theme) -> Result<I3Item> {
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
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let mut net = net_subscribe().await?;
        let mut enabled = self.only_on.is_empty();
        loop {
            // update item
            if enabled {
                ctx.update_item(self.item(&ctx.config.theme).await?).await?;
            }

            tokio::select! {
                // any bar event
                _ = ctx.wait_for_event(self.interval) => {
                    // don't update if disabled
                    if !enabled {
                        continue;
                    }
                },
                // network update - check update disabled state
                Ok(interfaces) = net.wait_for_change() => {
                    // if none of the filters matched
                    if interfaces.filtered(&self.only_on).is_empty() {
                        // if the item wasn't disabled, then empty it out
                        if enabled {
                            ctx.update_item(I3Item::empty()).await?;
                        }

                        // and set it to disabled
                        enabled = false;

                        // reset loop and wait to be enabled
                        continue;
                    }
                }
            }
        }
    }
}
