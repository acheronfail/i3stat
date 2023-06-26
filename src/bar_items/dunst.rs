use crate::error::Result;

use async_trait::async_trait;
use futures::StreamExt;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarItem, Context, StopAction};
use crate::dbus::dunst::DunstProxy;
use crate::dbus::{dbus_connection, BusType};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Dunst {}

impl Dunst {
    fn item(theme: &Theme, paused: bool) -> I3Item {
        if paused {
            I3Item::new("   ")
                .markup(I3Markup::Pango)
                .color(theme.bg)
                .background_color(theme.yellow)
        } else {
            I3Item::empty()
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Dunst {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        // get initial state
        let connection = dbus_connection(BusType::Session).await?;
        let dunst_proxy = DunstProxy::new(&connection).await?;
        let _ = ctx
            .update_item(Dunst::item(&ctx.config.theme, dunst_proxy.paused().await?))
            .await;

        // listen for changes
        let mut stream = dunst_proxy.receive_paused_changed().await;
        loop {
            tokio::select! {
                Some(change) = stream.next() => {
                    let paused = change.get().await?;
                    let _ = ctx.update_item(Dunst::item(&ctx.config.theme, paused)).await;
                },
                Some(_) = ctx.wait_for_event(None) => {
                    let paused = dunst_proxy.paused().await?;
                    let _ = ctx.update_item(Dunst::item(&ctx.config.theme, paused)).await;
                }
            }
        }
    }
}
