use std::error::Error;
use std::net::{SocketAddrV4, SocketAddrV6};
use std::time::Duration;

use async_trait::async_trait;
use iwlib::{get_wireless_info, WirelessInfo};
use nix::ifaddrs::getifaddrs;
use nix::net::if_::InterfaceFlags;

use crate::context::{BarItem, Context};
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::theme::Theme;
use crate::BarEvent;

#[derive(Debug)]
struct Interface {
    name: String,
    addr: String,
    is_wireless: Option<bool>,
}

impl Interface {
    fn new(name: impl AsRef<str>, addr: impl AsRef<str>) -> Interface {
        Interface {
            name: name.as_ref().into(),
            addr: addr.as_ref().into(),
            is_wireless: None,
        }
    }

    fn format_wireless(&self, i: WirelessInfo, theme: &Theme) -> (String, String) {
        let fg = match i.wi_quality {
            80..u8::MAX => theme.success,
            60..80 => theme.warning,
            40..60 => theme.danger,
            _ => theme.error,
        };

        (
            // interface details
            format!("({}) {}% at {}", self.addr, i.wi_quality, i.wi_essid),
            // colour
            fg.to_string(),
        )
    }

    fn format_normal(&self) -> (String, String) {
        (format!("({})", self.addr), "".into())
    }

    fn format(&mut self, theme: &Theme) -> (String, String) {
        // TODO: contribute AsRef upstream to https://github.com/psibi/iwlib-rs
        let name = self.name.clone();

        // check if this is a wireless network
        let (addr, fg) = match self.is_wireless {
            // not a wireless interface, just return defaults
            Some(false) => self.format_normal(),
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
                    self.format_normal()
                }
            },
        };

        (
            format!(r#"<span foreground="{}">{}{}</span>"#, fg, self.name, addr),
            format!(r#"<span foreground="{}">{}</span>"#, fg, self.name),
        )
    }
}

pub struct Nic {
    interval: Duration,
}

impl Default for Nic {
    fn default() -> Self {
        Nic {
            interval: Duration::from_secs(60),
        }
    }
}

impl Nic {
    fn get_interfaces() -> Vec<Interface> {
        let if_addrs = match getifaddrs() {
            Ok(if_addrs) => if_addrs,
            Err(_) => todo!(),
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

            let addr = match if_addr.address {
                Some(addr) => addr,
                None => continue,
            };

            let addr = match (addr.as_sockaddr_in(), addr.as_sockaddr_in6()) {
                (Some(ipv4), _) => format!("{}", SocketAddrV4::from(*ipv4).ip()),
                (_, Some(ipv6)) => format!("{}", SocketAddrV6::from(*ipv6).ip()),
                (None, None) => continue,
            };

            interfaces.push(Interface::new(if_addr.interface_name, addr));
        }

        interfaces
    }
}

#[async_trait(?Send)]
impl BarItem for Nic {
    // TODO: is there an agnostic/kernel way to detect network changes and _only then_ check for ips?
    // if not, then: dbus? networkmanager?
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        let mut idx = 0;
        loop {
            let mut interfaces = Nic::get_interfaces();
            let len = interfaces.len();
            idx = idx % len;

            let (full, short) = interfaces[idx].format(&ctx.theme);
            let full = if len > 1 {
                format!(
                    r#"{} <span foreground="{}">({}/{})</span>"#,
                    full,
                    ctx.theme.dark4,
                    idx + 1,
                    len
                )
            } else {
                full
            };

            let item = I3Item::new(full)
                .short_text(short)
                .name("nic")
                .markup(I3Markup::Pango);
            ctx.update_item(item).await?;

            // cycle through networks on click
            ctx.delay_with_event_handler(self.interval, |event| {
                if let BarEvent::Click(click) = event {
                    match click.button {
                        I3Button::Left | I3Button::ScrollUp => idx += 1,
                        I3Button::Right | I3Button::ScrollDown => {
                            if idx == 0 {
                                idx = len - 1
                            } else {
                                idx -= 1
                            }
                        }
                        _ => {}
                    }
                }

                async {}
            })
            .await;
        }
    }
}
