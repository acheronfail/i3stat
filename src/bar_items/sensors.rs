use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use sysinfo::{ComponentExt, SystemExt};
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize)]
pub struct Sensors {
    #[serde(with = "humantime_serde")]
    interval: Duration,
    label: String,
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
        loop {
            let temp = {
                let mut state = ctx.state.lock().unwrap();
                // TODO: store list of references to Components, so don't have to iter each time
                state
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
                    })
                    .unwrap()
            };

            let (icon, color) = Self::get_icon(&ctx.theme, temp as u32);
            let mut item = I3Item::new(format!("{} {:.0}°C", icon, temp))
                .short_text(format!("{:.0}C", temp))
                .name("sensors");

            if let Some(color) = color {
                item = item.color(color);
            }

            ctx.update_item(item).await?;
            sleep(self.interval).await;
        }
    }
}
