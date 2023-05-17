mod generated;

use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use dbus::arg::Variant;
use dbus::message::MatchRule;
use dbus::nonblock;
use generated::OrgDunstprojectCmd0;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarEvent, BarItem, Context};
use crate::dbus::BusType;
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

#[async_trait(?Send)]
impl BarItem for Dunst {
    fn register_dbus_interest(&self) -> Option<(BusType, MatchRule<'static>)> {
        Some((
            BusType::Session,
            MatchRule::new()
                .with_type(dbus::MessageType::MethodCall)
                .with_path("/org/freedesktop/Notifications")
                .with_interface("org.freedesktop.DBus.Properties")
                .with_member("Set"),
        ))
    }

    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        // get initial paused state
        {
            let con = ctx.state.borrow().get_dbus_connection()?;
            let dunst_proxy = nonblock::Proxy::new(
                "org.freedesktop.Notifications",
                "/org/freedesktop/Notifications",
                Duration::from_secs(5),
                con,
            );
            match dunst_proxy.paused().await {
                Ok(paused) => {
                    ctx.update_item(Dunst::item(&ctx.theme, paused))
                        .await
                        .unwrap();
                }
                Err(e) => log::error!("failed to get initial paused state: {}", e),
            }
        }

        // wait for changes on dbus
        loop {
            if let Some(BarEvent::DbusMessage(msg)) = ctx.wait_for_event().await {
                match msg.read3::<&str, &str, Variant<bool>>() {
                    Ok((_, what, is_paused)) => {
                        if what == "paused" {
                            ctx.update_item(Dunst::item(&ctx.theme, is_paused.0))
                                .await?
                        }
                    }
                    Err(e) => {
                        log::error!("failed to read dbus message: {}", e);
                        // TODO: signal here back to dbus to return `false` and stop listening for events
                    }
                }
            }
        }
    }
}
