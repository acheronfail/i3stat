use crate::dbus::mako::MakoProxy;
use crate::dbus::{dbus_connection, BusType};
use crate::error::Result;

use async_trait::async_trait;
use clap::Parser;
use futures::StreamExt;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;

use crate::context::{BarEvent, BarItem, Context, CustomResponse, StopAction};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

const MAKO_DND_MODE: &str = "do-not-disturb";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Mako {}

impl Mako {
    fn item(theme: &Theme, dnd_on: bool) -> I3Item {
        if dnd_on {
            I3Item::new("   ")
                .markup(I3Markup::Pango)
                .color(theme.bg)
                .background_color(theme.yellow)
        } else {
            I3Item::empty()
        }
    }

    fn contains_dnd_mode(stdout: &str) -> bool {
        stdout.trim().lines().any(|line| line == MAKO_DND_MODE)
    }

    async fn dnd_enabled() -> Result<bool> {
        let output = Command::new("makoctl").arg("mode").output().await?;
        Ok(Mako::contains_dnd_mode(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }

    async fn dnd_toggle() -> Result<bool> {
        let output = Command::new("makoctl")
            .args(&["mode", "-t", MAKO_DND_MODE])
            .output()
            .await?;

        Ok(Mako::contains_dnd_mode(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }

    async fn dnd_set(enable: bool) -> Result<bool> {
        let output = Command::new("makoctl")
            .args(&["mode", if enable { "-a" } else { "-r" }, MAKO_DND_MODE])
            .output()
            .await?;

        Ok(Mako::contains_dnd_mode(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }
}

#[async_trait(?Send)]
impl BarItem for Mako {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let connection = dbus_connection(BusType::Session).await?;
        let mako_proxy = MakoProxy::new(connection).await?;
        let mut stream = mako_proxy.receive_modes_changed().await;

        // TODO: when `mako` releases this https://github.com/emersion/mako/pull/552
        // we'll be able to subscribe to events, but until then just call out to `makoctl` each time
        // we're able to setup the dbus listener now, but we should remove the `makoctl` calls later
        let _ = ctx
            .update_item(Mako::item(&ctx.config.theme, Mako::dnd_enabled().await?))
            .await?;

        loop {
            tokio::select! {
                Some(change) = stream.next() => {
                    let dnd_on = change.get().await?.iter().any(|line| line == MAKO_DND_MODE);
                    let _ = ctx.update_item(Mako::item(&ctx.config.theme, dnd_on)).await;
                }

                Some(ev) = ctx.wait_for_event(None) => {
                    if let BarEvent::Custom { payload, responder } = ev {
                        let resp = match MakoCommand::try_parse_from(payload) {
                            Ok(cmd) => {
                                match match cmd {
                                    MakoCommand::Toggle => Mako::dnd_toggle().await,
                                    MakoCommand::Set { on } => Mako::dnd_set(on).await,
                                } {
                                    Ok(enabled) => CustomResponse::Json(json!({ "enabled": enabled })),
                                    Err(e) => CustomResponse::Json(json!({ "failure": e.to_string() })),
                                }
                            }
                            Err(e) => CustomResponse::Help(e.render()),
                        };

                        let _ = responder.send(resp);

                    }
                    let _ = ctx.update_item(Mako::item(&ctx.config.theme, Mako::dnd_enabled().await?)).await?;
                }
            }
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "mako", no_binary_name = true)]
enum MakoCommand {
    /// Toggle Do Not Disturb mode
    Toggle,
    /// Set Do Not Disturb
    Set { on: bool },
}
