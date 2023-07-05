use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::error::Result;
use crate::i3::{I3Item, I3Markup, I3Modifier};
use crate::theme::Theme;
use crate::util::filter::InterfaceFilter;
use crate::util::{net_subscribe, NetlinkInterface, Paginator};

struct Connection {
    // TODO: borrow?
    name: String,
    addr: String,
    // TODO: if wireless, refresh at an interval?
    // FIXME: compute only when needed, not all the time
    detail: Option<String>,
    fg: HexColor,
}

impl Connection {
    pub fn format(&self, _theme: &Theme) -> (String, String) {
        let fg = format!(r#" foreground="{}""#, self.fg);
        (
            format!(
                r#"<span{}>{}({}){}</span>"#,
                fg,
                self.name,
                self.addr,
                match &self.detail {
                    Some(detail) => format!(" {}", detail),
                    None => "".into(),
                }
            ),
            format!(r#"<span{}>{}</span>"#, fg, self.name),
        )
    }
}

async fn connections_from_interfaces(
    theme: &Theme,
    interfaces: Vec<NetlinkInterface>,
) -> Result<Vec<Connection>> {
    let mut result = vec![];
    for interface in interfaces {
        for addr in &interface.ip_addresses {
            let wireless_info = interface.wireless_info().await;
            result.push(Connection {
                fg: wireless_info
                    .as_ref()
                    .and_then(|info| info.signal.as_ref())
                    .map_or(theme.green, |signal| match signal.quality as u8 {
                        100..=u8::MAX => theme.green,
                        80..=99 => theme.green,
                        60..=79 => theme.yellow,
                        40..=59 => theme.orange,
                        _ => theme.red,
                    }),
                name: interface.name.to_string(),
                addr: addr.to_string(),
                detail: wireless_info.map(|info| match (info.ssid, info.signal) {
                    (Some(ssid), Some(signal)) => format!("{:.0}% at {}", signal.quality, ssid),
                    (Some(ssid), None) => ssid.to_string(),
                    _ => "".into(),
                }),
            });
        }
    }

    Ok(result)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Nic {
    #[serde(default, with = "crate::human_time::option")]
    interval: Option<Duration>,
    #[serde(default)]
    filter: Vec<InterfaceFilter>,
}

#[async_trait(?Send)]
impl BarItem for Nic {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let mut net = net_subscribe().await?;
        let mut p = Paginator::new();

        let mut interfaces = vec![];
        loop {
            tokio::select! {
                // wait for network changes
                Ok(list) = net.wait_for_change() => {
                    interfaces = connections_from_interfaces(&ctx.config.theme, list.filtered(&self.filter)).await?;
                },
                // on any bar event
                Some(event) = ctx.wait_for_event(self.interval) => {
                    // update paginator
                    p.update(&event);

                    // request interfaces update
                    if let BarEvent::Click(click) = event {
                        if click.modifiers.contains(&I3Modifier::Control) {
                            net.trigger_update().await?;
                        }
                    }
                }
            }

            let item = if interfaces.is_empty() {
                // TODO: differentiate between empty after filtering, and completely disconnected?
                I3Item::new("inactive").color(ctx.config.theme.dim)
            } else {
                p.set_len(interfaces.len());

                let theme = &ctx.config.theme;
                let (full, short) = interfaces[p.idx()].format(theme);
                let full = format!(r#"{}{}"#, full, p.format(theme));

                I3Item::new(full).short_text(short).markup(I3Markup::Pango)
            };

            ctx.update_item(item).await?;
        }
    }
}
