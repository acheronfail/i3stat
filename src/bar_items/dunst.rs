use std::error::Error;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarItem, Context};
use crate::dbus::dbus_connection;
use crate::dbus::dunst::DunstProxy;
use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Dunst {}

impl Dunst {
    fn item(theme: &Theme, paused: bool) -> I3Item {
        I3Item::new(if paused { "   " } else { "" })
            .color(theme.dark1)
            .background_color(theme.warning)
    }
}

#[async_trait(?Send)]
impl BarItem for Dunst {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        // this item doesn't receive any input, so close the receiver
        ctx.raw_event_rx().close();

        // get initial state
        let connection = dbus_connection(crate::dbus::BusType::Session).await?;
        let dunst_proxy = DunstProxy::new(&connection).await?;
        let _ = ctx
            .update_item(Dunst::item(&ctx.theme, dunst_proxy.paused().await?))
            .await;

        // listen for changes
        let mut stream = dunst_proxy.receive_paused_changed().await;
        loop {
            tokio::select! {
                Some(change) = stream.next() => {
                    let paused = change.get().await?;
                    let _ = ctx.update_item(Dunst::item(&ctx.theme, paused)).await;
                },
                Some(_) = ctx.wait_for_event(None) => {
                    let paused = dunst_proxy.paused().await?;
                    let _ = ctx.update_item(Dunst::item(&ctx.theme, paused)).await;
                }
            }
        }
    }
}
