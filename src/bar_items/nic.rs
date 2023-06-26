use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use iwlib::WirelessInfo;
use serde_derive::{Deserialize, Serialize};

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::i3::{I3Item, I3Markup, I3Modifier};
use crate::theme::Theme;
use crate::util::filter::InterfaceFilter;
use crate::util::net::Interface;
use crate::util::{net_subscribe, Paginator};

impl Interface {
    fn format_wireless(&self, i: WirelessInfo, theme: &Theme) -> (String, Option<HexColor>) {
        let fg = match i.wi_quality {
            100..=u8::MAX => theme.green,
            80..=99 => theme.green,
            60..=79 => theme.yellow,
            40..=59 => theme.orange,
            _ => theme.red,
        };

        (
            format!("({}) {}% at {}", self.addr, i.wi_quality, i.wi_essid),
            Some(fg),
        )
    }

    fn format_normal(&self, theme: &Theme) -> (String, Option<HexColor>) {
        (format!("({})", self.addr), Some(theme.green))
    }

    fn format(&self, theme: &Theme) -> (String, String) {
        let (addr, fg) = match self.get_wireless_info() {
            Some(info) => self.format_wireless(info, theme),
            None => self.format_normal(theme),
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Nic {
    #[serde(default, with = "crate::human_time::option")]
    interval: Option<Duration>,
    #[serde(default)]
    filter: Vec<InterfaceFilter>,
}

#[async_trait(?Send)]
impl BarItem for Nic {
    async fn start(&self, mut ctx: Context) -> Result<StopAction, Box<dyn Error>> {
        let mut net = net_subscribe().await?;
        let mut p = Paginator::new();

        let mut interfaces = vec![];
        loop {
            tokio::select! {
                // wait for network changes
                Ok(list) = net.wait_for_change() => {
                    interfaces = list.filtered(&self.filter);
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
