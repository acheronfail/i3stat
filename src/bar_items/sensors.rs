use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{ComponentExt, SystemExt};
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::format::{float, FloatFormat};
use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Sensors {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    label: String,
    #[serde(flatten)]
    float_fmt: FloatFormat,
}

impl Sensors {
    fn get_icon(theme: &Theme, temp: u32) -> (&'static str, Option<HexColor>) {
        match temp {
            0..=59 => ("", None),
            60..=69 => ("", Some(theme.warning)),
            70..=79 => ("", Some(theme.warning)),
            80..=89 => ("", Some(theme.danger)),
            90..=u32::MAX => ("", Some(theme.error)),
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Sensors {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        {
            ctx.state.borrow_mut().sys.refresh_components_list();
        }

        loop {
            let temp = {
                let search = ctx
                    .state
                    .borrow_mut()
                    .sys
                    .components_mut()
                    .iter_mut()
                    .find_map(|c| {
                        if c.label() == self.label {
                            c.refresh();
                            Some(c.temperature())
                        } else {
                            None
                        }
                    });

                match search {
                    Some(temp) => temp,
                    None => {
                        break Err(format!("no component found with label: {}", self.label).into())
                    }
                }
            };

            let (icon, color) = Self::get_icon(&ctx.theme, temp as u32);
            let temp = float(temp, &self.float_fmt);
            let mut item =
                I3Item::new(format!("{} {}°C", icon, temp)).short_text(format!("{}C", temp));

            if let Some(color) = color {
                item = item.color(color);
            }

            ctx.update_item(item).await?;
            sleep(self.interval).await;
        }
    }
}
