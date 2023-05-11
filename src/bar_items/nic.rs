use std::cmp::Ordering;
use std::error::Error;
use std::net::{SocketAddrV4, SocketAddrV6};
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use iwlib::{get_wireless_info, WirelessInfo};
use nix::ifaddrs::getifaddrs;
use nix::net::if_::InterfaceFlags;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarItem, Context};
use crate::format::fraction;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

#[derive(Debug, PartialEq, Eq)]
struct Interface {
    name: String,
    addr: String,
    is_wireless: Option<bool>,
}

impl PartialOrd for Interface {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.name.partial_cmp(&other.name) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        match self.addr.partial_cmp(&other.addr) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        self.is_wireless.partial_cmp(&other.is_wireless)
    }
}

impl Ord for Interface {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl Interface {
    fn new(name: impl AsRef<str>, addr: impl AsRef<str>) -> Interface {
        Interface {
            name: name.as_ref().into(),
            addr: addr.as_ref().into(),
            is_wireless: None,
        }
    }

    fn format_wireless(&self, i: WirelessInfo, theme: &Theme) -> (String, Option<HexColor>) {
        let fg = match i.wi_quality {
            80..u8::MAX => theme.success,
            60..80 => theme.warning,
            40..60 => theme.danger,
            _ => theme.error,
        };

        (
            format!("({}) {}% at {}", self.addr, i.wi_quality, i.wi_essid),
            Some(fg),
        )
    }

    fn format_normal(&self, theme: &Theme) -> (String, Option<HexColor>) {
        (format!("({})", self.addr), Some(theme.success))
    }

    fn format(&mut self, theme: &Theme) -> (String, String) {
        // TODO: contribute AsRef upstream to https://github.com/psibi/iwlib-rs
        let name = self.name.clone();

        // check if this is a wireless network
        let (addr, fg) = match self.is_wireless {
            // not a wireless interface, just return defaults
            Some(false) => self.format_normal(theme),
            // SAFETY: we've previously checked if this is a wireless network
            Some(true) => self.format_wireless(get_wireless_info(name).unwrap(), theme),
            // check if we're a wireless network and remember for next time
            None => match get_wireless_info(name) {
                Some(i) => {
                    self.is_wireless = Some(true);
                    self.format_wireless(i, theme)
                }
                None => {
                    self.is_wireless = Some(false);
                    self.format_normal(theme)
                }
            },
        };

        let fg = fg
            .map(|c| format!(r#" foreground="{}""#, c))
            .unwrap_or("".into());
        (
            format!(r#"<span{}>{}{}</span>"#, fg, self.name, addr),
            format!(r#"<span{}>{}</span>"#, fg, self.name),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Nic {
    #[serde(with = "humantime_serde")]
    interval: Duration,
}

impl Nic {
    fn get_interfaces() -> Result<Vec<Interface>, Box<dyn Error>> {
        let if_addrs = match getifaddrs() {
            Ok(if_addrs) => if_addrs,
            Err(e) => return Err(format!("call to `getifaddrs` failed: {}", e).into()),
        };

        let mut interfaces = vec![];
        for if_addr in if_addrs.into_iter() {
            // skip any interfaces that aren't active
            if !if_addr.flags.contains(InterfaceFlags::IFF_UP) {
                continue;
            }

            // skip the local loopback interface
            if if_addr.flags.contains(InterfaceFlags::IFF_LOOPBACK) {
                continue;
            }

            // skip any unsupported entry (see nix's `getifaddrs` documentation)
            let addr = match if_addr.address {
                Some(addr) => addr,
                None => continue,
            };

            // extract ip address
            let addr = match (addr.as_sockaddr_in(), addr.as_sockaddr_in6()) {
                (Some(ipv4), _) => format!("{}", SocketAddrV4::from(*ipv4).ip()),
                (_, Some(ipv6)) => format!("{}", SocketAddrV6::from(*ipv6).ip()),
                (None, None) => continue,
            };

            interfaces.push(Interface::new(if_addr.interface_name, addr));
        }

        interfaces.sort();

        Ok(interfaces)
    }
}

#[async_trait(?Send)]
impl BarItem for Nic {
    // TODO: is there an agnostic/kernel way to detect network changes and _only then_ check for ips?
    // kernel-userspace api would be: netlink, see: https://stackoverflow.com/a/2353441/5552584
    //  also: https://inai.de/documents/Netlink_Protocol.pdf
    //  also: https://github.com/mullvad/mnl-rs
    // fallback dbus: `dbus-monitor --system "type='signal',interface='org.freedesktop.NetworkManager'"`
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        let mut idx = 0;
        loop {
            let mut interfaces = Nic::get_interfaces()?;
            let len = interfaces.len();
            idx = idx % len;

            let (full, short) = interfaces[idx].format(&ctx.theme);
            let full = format!(r#"{}{}"#, full, fraction(&ctx.theme, idx + 1, len));

            let item = I3Item::new(full)
                .short_text(short)
                .name("nic")
                .markup(I3Markup::Pango);
            ctx.update_item(item).await?;

            // cycle through networks on click
            ctx.delay_with_event_handler(self.interval, |event| {
                Context::paginate(&event, len, &mut idx);
                async {}
            })
            .await;
        }
    }
}
