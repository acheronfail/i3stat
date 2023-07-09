use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::error::Result;
use crate::i3::{I3Item, I3Markup, I3Modifier};
use crate::theme::Theme;
use crate::util::filter::InterfaceFilter;
use crate::util::nl80211::SignalStrength;
use crate::util::{net_subscribe, Interfaces, NetlinkInterface, Paginator};

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
enum WirelessDisplay {
    Hidden,
    #[default]
    Percent,
    Dbm,
}

enum ConnectionDetail {
    None,
    Ssid(String),
    SsidAndSignal(String, SignalStrength),
}

impl ConnectionDetail {
    fn display(&self, wireless_display: WirelessDisplay) -> String {
        if matches!(wireless_display, WirelessDisplay::Hidden) {
            return "".into();
        }

        match self {
            ConnectionDetail::SsidAndSignal(ssid, signal) => {
                let signal = match wireless_display {
                    WirelessDisplay::Percent => format!("{}%", signal.quality() as u8),
                    WirelessDisplay::Dbm => format!("{} dBm", signal.dbm),
                    // SAFETY: we match and early return on this at the beginning of this function
                    WirelessDisplay::Hidden => unreachable!(),
                };
                format!("{signal} at {ssid}", ssid = ssid, signal = signal)
            }
            ConnectionDetail::Ssid(ssid) => ssid.into(),
            ConnectionDetail::None => "".into(),
        }
    }
}

struct Connection<'a> {
    /// Interface name
    name: &'a str,
    /// Interface address as a string
    addr: &'a IpAddr,
    /// Extra detail about the connection
    detail: Option<ConnectionDetail>,
    /// Connection quality expressed as a percentage value between 0 and 100
    /// Only set when connection is wireless, and expresses the signal strength
    /// This is used to infer which colour the item should be
    quality: Option<u8>,
}

impl<'a> Connection<'a> {
    async fn new(interface: &'a NetlinkInterface, addr: &'a IpAddr) -> Connection<'a> {
        let wireless_info = interface.wireless_info().await;
        let quality = wireless_info
            .as_ref()
            .and_then(|info| info.signal.as_ref())
            .map(|signal| signal.quality() as u8);

        Connection {
            name: &interface.name,
            addr: &addr,
            detail: wireless_info.map(|info| match (info.ssid, info.signal) {
                (Some(ssid), Some(signal)) => {
                    ConnectionDetail::SsidAndSignal(ssid.to_string(), signal)
                }
                (Some(ssid), None) => ConnectionDetail::Ssid(ssid.to_string()),
                _ => ConnectionDetail::None,
            }),
            quality,
        }
    }

    fn format(&self, theme: &Theme, wireless_display: WirelessDisplay) -> (String, String) {
        let fg = format!(
            r#" foreground="{}""#,
            match self.quality {
                Some(quality) => match quality {
                    100..=u8::MAX => theme.green,
                    80..=99 => theme.green,
                    60..=79 => theme.yellow,
                    40..=59 => theme.orange,
                    _ => theme.red,
                },
                None => theme.green,
            }
        );
        (
            format!(
                r#"<span{}>{}({}){}</span>"#,
                fg,
                self.name,
                self.addr,
                match (wireless_display, &self.detail) {
                    (WirelessDisplay::Hidden, _) | (_, None) => "".into(),
                    (_, Some(detail)) => format!(" {}", detail.display(wireless_display)),
                }
            ),
            format!(r#"<span{}>{}</span>"#, fg, self.name),
        )
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Nic {
    #[serde(default, with = "crate::human_time::option")]
    interval: Option<Duration>,
    #[serde(default)]
    filter: Vec<InterfaceFilter>,
    #[serde(default)]
    wireless_display: WirelessDisplay,
    #[serde(default, with = "crate::human_time::option")]
    wireless_refresh_interval: Option<Duration>,
}

#[async_trait(?Send)]
impl BarItem for Nic {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let wireless_refresh_trigger = || async {
            match (self.wireless_display, self.wireless_refresh_interval) {
                (WirelessDisplay::Hidden, _) | (_, None) => futures::future::pending::<()>().await,
                (_, Some(duration)) => tokio::time::sleep(duration).await,
            }
        };

        let mut net = net_subscribe().await?;
        let mut p = Paginator::new();

        let mut interfaces = Interfaces::default();
        let mut total_address_count = interfaces.len_addresses();
        loop {
            tokio::select! {
                // wait for network changes
                Ok(new_interfaces) = net.wait_for_change() => {
                    total_address_count = new_interfaces.len_addresses();
                    interfaces = new_interfaces.filtered(&self.filter);
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
                // if set, start a timeout to refresh the wireless details
                // this just breaks the `select!` so the wireless details will be fetched again
                () = wireless_refresh_trigger() => {}
            }

            let item = if interfaces.is_empty() {
                if total_address_count > 0 {
                    I3Item::new(format!("filtered: {}", total_address_count))
                } else {
                    I3Item::new("disconnected")
                }
                .color(ctx.config.theme.dim)
            } else {
                p.set_len(interfaces.len_addresses())?;
                let theme = &ctx.config.theme;
                // SAFETY(unwrap): we always set the paginator's length to `len_addresses` so it
                // should always be within bounds
                let (interface, ip_addr) = interfaces.get_address_at(p.idx()).unwrap();
                let (full, short) = Connection::new(interface, ip_addr)
                    .await
                    .format(theme, self.wireless_display);

                let full = format!(r#"{}{}"#, full, p.format(theme));
                I3Item::new(full).short_text(short).markup(I3Markup::Pango)
            };

            ctx.update_item(item).await?;
        }
    }
}
