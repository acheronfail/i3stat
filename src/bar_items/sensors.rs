use crate::error::Result;
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::Components;
use tokio::time::sleep;

use crate::context::{BarItem, Context, StopAction};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::format::{float, FloatFormat};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Sensors {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    #[serde(default)]
    label: Option<String>,
    component: String,
    #[serde(flatten)]
    float_fmt: FloatFormat,
}

impl Sensors {
    fn get_icon(theme: &Theme, temp: u32) -> (&'static str, Option<HexColor>) {
        match temp {
            0..=59 => ("", None),
            60..=69 => ("", Some(theme.yellow)),
            70..=79 => ("", Some(theme.yellow)),
            80..=89 => ("", Some(theme.orange)),
            90..=u32::MAX => ("", Some(theme.red)),
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Sensors {
    async fn start(&self, ctx: Context) -> Result<StopAction> {
        let mut components = Components::new_with_refreshed_list();

        let label = self.label.as_deref().unwrap_or("");
        loop {
            let temp = {
                let search = components.iter_mut().find_map(|c| {
                    if c.label() == self.component {
                        c.refresh();
                        Some(c.temperature())
                    } else {
                        None
                    }
                });

                match search {
                    Some(Some(temp)) => temp,
                    Some(None) | None => {
                        break Err(
                            format!("no component found with name: {}", self.component).into()
                        )
                    }
                }
            };

            let (icon, color) = Self::get_icon(&ctx.config.theme, temp as u32);
            let temp = float(temp, &self.float_fmt);
            let mut item = I3Item::new(format!("{} {}°C{}", icon, temp, label))
                .short_text(format!("{}C", temp))
                .markup(I3Markup::Pango);

            if let Some(color) = color {
                item = item.color(color);
            }

            ctx.update_item(item).await?;
            sleep(self.interval).await;
        }
    }
}
