//! An item which interfaces with https://github.com/tuxedocomputers/tuxedo-control-center

use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::dbus::tuxedo::TccdProxy;
use crate::dbus::{dbus_connection, BusType};
use crate::error::Result;
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::util::Paginator;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Tuxedo {}

#[async_trait(?Send)]
impl BarItem for Tuxedo {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let tccd = TccdProxy::new(dbus_connection(BusType::System).await?).await?;
        let mut p = Paginator::new();
        let mut first = true;
        loop {
            let profiles = dbg!(tccd.get_profiles().await?);
            let _ = p.set_len(profiles.len());
            if first {
                if let Some(idx) = profiles.iter().position(|p| p.active) {
                    let _ = p.set_idx(idx);
                    first = false;
                }
            }

            let display_profile = &profiles[p.idx()];
            ctx.update_item(
                I3Item::new(format!(
                    "{} {}",
                    &display_profile.name,
                    p.format(&ctx.config.theme)
                ))
                .color(if display_profile.active {
                    ctx.config.theme.green
                } else {
                    ctx.config.theme.fg
                })
                .markup(I3Markup::Pango),
            )
            .await?;

            match ctx.wait_for_event(None).await {
                Some(bar_event) => {
                    p.update(&bar_event);
                    if let BarEvent::Click(event) = bar_event {
                        if event.button == I3Button::Left {
                            // FIXME: this doesn't work, see comments on method
                            tccd.set_temp_profile(&display_profile.name).await?;
                        }
                    }
                }
                None => {
                    // anything else just refreshes the item
                }
            }
        }
    }
}
