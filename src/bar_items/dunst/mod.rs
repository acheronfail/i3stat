mod generated;

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dbus::arg::Variant;
use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus::nonblock::SyncConnection;
use dbus::{nonblock, Message};
use generated::OrgDunstprojectCmd0;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::context::{BarItem, Context};
use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize)]
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
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        let ctx = Arc::new(ctx);

        // connect to dbus
        let (resource, con) = dbus_tokio::connection::new_session_sync()?;
        let (exit_tx, mut exit_rx) = mpsc::channel(1);
        tokio::spawn(async move {
            let _ = exit_tx.send(resource.await).await;
        });

        // get initial paused state
        let dunst_proxy = nonblock::Proxy::new(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            Duration::from_secs(5),
            con.clone(),
        );
        let paused = dunst_proxy.paused().await?;
        ctx.update_item(Dunst::item(&ctx.theme, paused)).await?;

        // setup a monitor to watch for changes
        let rule = MatchRule::new()
            .with_type(dbus::MessageType::MethodCall)
            .with_path("/org/freedesktop/Notifications")
            .with_interface("org.freedesktop.DBus.Properties")
            .with_member("Set");

        let dbus_proxy = nonblock::Proxy::new(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_secs(5),
            con.clone(),
        );

        // tell dbus we're going to become a monitor
        // https://dbus.freedesktop.org/doc/dbus-specification.html#bus-messages-become-monitor
        let _: () = dbus_proxy
            .method_call(
                "org.freedesktop.DBus.Monitoring",
                "BecomeMonitor",
                (vec![rule.match_str()], 0u32),
            )
            .await?;

        // TODO: is there an "async" way to stream response from a monitor? (rather than this hack)
        // See: https://github.com/diwic/dbus-rs/issues/431
        let (msg_tx, mut msg_rx) = mpsc::channel(8);
        con.start_receive(
            rule.clone(),
            Box::new(move |msg: Message, _con: &SyncConnection| {
                match msg.read3::<&str, &str, Variant<bool>>() {
                    Ok((_, what, is_paused)) => {
                        if what == "paused" {
                            let tx = msg_tx.clone();
                            tokio::spawn(async move {
                                let _ = tx.send(is_paused.0).await;
                            });
                        }

                        // false to continue receiving events
                        true
                    }
                    Err(e) => {
                        log::error!("failed to read dbus message: {}", e);

                        // false to stop receiving events
                        false
                    }
                }
            }),
        );

        loop {
            tokio::select! {
                // update item on dbus messages
                Some(paused) = msg_rx.recv() => {
                    ctx.update_item(Dunst::item(&ctx.theme, paused)).await?
                }
                // exit on error
                Some(err) = exit_rx.recv() => {
                    break Err(format!("unexpected disconnect from dbus: {}", err).into())
                }
            }
        }
    }
}
