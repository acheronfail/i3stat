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
        tokio::spawn(async move {
            // TODO: handle, rather than panicking
            panic!("Lost connecton to dbus: {}", resource.await);
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
        let (tx, mut rx) = mpsc::channel(8);
        con.start_receive(
            rule.clone(),
            Box::new(move |msg: Message, _con: &SyncConnection| {
                let (_, what, is_paused): (&str, &str, Variant<bool>) = msg.read3().unwrap();
                if what == "paused" {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        tx.send(is_paused.0).await.unwrap();
                    });
                }

                true
            }),
        );

        loop {
            match rx.recv().await {
                Some(paused) => ctx.update_item(Dunst::item(&ctx.theme, paused)).await?,
                None => {}
            }
        }
    }
}
