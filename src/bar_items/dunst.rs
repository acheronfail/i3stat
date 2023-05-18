use std::error::Error;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde_derive::{Deserialize, Serialize};
use zbus::dbus_proxy;

use crate::context::{BarItem, Context};
use crate::dbus::dbus_connection;
use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Dunst {}

impl Dunst {
    fn item(theme: &Theme, paused: bool) -> I3Item {
        I3Item::new(if paused { "   " } else { "" })
            .color(theme.dark1)
            .background_color(theme.warning)
            .name("dunst")
    }
}

#[dbus_proxy(
    default_path = "/org/freedesktop/Notifications",
    default_service = "org.freedesktop.Notifications",
    interface = "org.dunstproject.cmd0",
    gen_blocking = false
)]
trait DunstDbus {
    #[dbus_proxy(property, name = "paused")]
    fn paused(&self) -> zbus::Result<bool>;
}

#[async_trait(?Send)]
impl BarItem for Dunst {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        // this item doesn't receive any input, so close the receiver
        ctx.raw_event_rx().close();

        // get initial state
        let connection = dbus_connection(crate::dbus::BusType::Session).await?;
        let dunst_proxy = DunstDbusProxy::new(&connection).await?;
        let _ = ctx
            .update_item(Dunst::item(&ctx.theme, dunst_proxy.paused().await?))
            .await;

        // listen for changes
        let mut stream = dunst_proxy.receive_paused_changed().await;
        while let Some(change) = stream.next().await {
            let paused = change.get().await?;
            let _ = ctx.update_item(Dunst::item(&ctx.theme, paused)).await;
        }

        Err("unexpected end of dbus stream".into())
    }
}
