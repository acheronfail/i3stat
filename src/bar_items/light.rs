//! An item which controls lights.
//! https://github.com/haikarainen/light has been a good inspiration, and could
//! be for future features (if things like razer devices should ever be supported, etc).

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clap::Parser;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;

use crate::context::{BarEvent, BarItem, Context, CustomResponse, StopAction};
use crate::error::Result;
use crate::i3::{I3Button, I3Item};

struct LightFile {
    /// Max brightness of this device
    max_brightness: u64,
    /// The file to read to or write from to get/set the brightness.
    brightness_file: PathBuf,
}

impl LightFile {
    async fn read_u64(path: impl AsRef<Path>) -> Result<u64> {
        Ok(fs::read_to_string(path.as_ref())
            .await?
            .trim()
            .parse::<u64>()?)
    }

    pub async fn new(path: impl AsRef<Path>) -> Result<LightFile> {
        let path = path.as_ref();

        let max_brightness_path = path.join("max_brightness");
        let max_brightness = Self::read_u64(&max_brightness_path).await?;

        let brightness_file = path.join("brightness");
        match brightness_file.exists() {
            true => Ok(LightFile {
                max_brightness,
                brightness_file,
            }),
            false => bail!("{}/brightness does not exist", path.display()),
        }
    }

    /// Get the brightness of this light as a percentage
    pub async fn get(&self) -> Result<u8> {
        let value = Self::read_u64(&self.brightness_file).await?;
        Ok(((value * 100 + self.max_brightness / 2) / self.max_brightness) as u8)
    }

    /// Set the brightness of this light to a percentage
    pub async fn set(&self, pct: u8) -> Result<()> {
        let step = self.max_brightness / 100;
        let value = (pct.clamp(0, 100) as u64) * step;
        fs::write(&self.brightness_file, value.to_string()).await?;

        Ok(())
    }

    pub async fn adjust(&self, amount: i8) -> Result<()> {
        let pct = self.get().await?;
        self.set(
            pct.saturating_add_signed(amount - (pct as i8 % amount))
                .clamp(0, 100),
        )
        .await
    }

    /// Detects what is most likely the default backlight.
    /// It does this by just looking for the backlight with the largest value for max_brightness.
    pub async fn detect() -> Result<LightFile> {
        // read all backlights
        let mut entries = fs::read_dir("/sys/class/backlight").await?;
        let mut backlights = vec![];
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            match Self::read_u64(path.join("max_brightness")).await {
                Ok(value) => backlights.push((path, value)),
                _ => continue,
            }
        }

        // sort by max brightness
        backlights.sort_unstable_by_key(|ref pair| pair.1);

        // return a light for the "brightest" backlight
        match backlights.last() {
            Some((path, _)) => LightFile::new(path).await,
            None => bail!("no backlights found"),
        }
    }

    pub async fn format(&self) -> Result<I3Item> {
        let pct = self.get().await?;
        let icon = match pct {
            0..=14 => "󰃚",
            15..=29 => "󰃛",
            30..=44 => "󰃜",
            45..=59 => "󰃝",
            60..=74 => "󰃞",
            75..=89 => "󰃟",
            90..=u8::MAX => "󰃠",
        };

        Ok(I3Item::new(format!("{} {:>3}%", icon, pct)))
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Light {
    /// Optional path to a specific light.
    path: Option<PathBuf>,
    /// How much to increment the light when scrolling up or down.
    /// Defaults to 5.
    increment: Option<u8>,
}

#[async_trait(?Send)]
impl BarItem for Light {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let light = match &self.path {
            Some(path) => LightFile::new(path).await?,
            None => LightFile::detect().await?,
        };

        let increment = self.increment.unwrap_or(5) as i8;
        loop {
            ctx.update_item(light.format().await?).await?;
            match ctx.wait_for_event(None).await {
                // mouse events
                Some(BarEvent::Click(click)) => match click.button {
                    I3Button::Left => light.set(1).await?,
                    I3Button::Right => light.set(100).await?,
                    I3Button::ScrollUp => light.adjust(increment).await?,
                    I3Button::ScrollDown => light.adjust(-increment).await?,
                    _ => {}
                },
                // custom ipc events
                Some(BarEvent::Custom { payload, responder }) => {
                    let resp = match LightCommand::try_parse_from(payload) {
                        Ok(cmd) => {
                            match match cmd {
                                LightCommand::Increase => light.adjust(increment).await,
                                LightCommand::Decrease => light.adjust(-increment).await,
                                LightCommand::Set { pct } => light.set(pct).await,
                            } {
                                Ok(()) => CustomResponse::Json(json!(())),
                                Err(e) => CustomResponse::Json(json!({
                                    "failure": e.to_string()
                                })),
                            }
                        }
                        Err(e) => CustomResponse::Help(e.render()),
                    };

                    let _ = responder.send(resp);
                }
                // other events just trigger a refresh
                _ => {}
            }
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "light", no_binary_name = true)]
enum LightCommand {
    /// Increase the brightness by the configured increment amount
    Increase,
    /// Decrease the brightness by the configured increment amount
    Decrease,
    /// Set the brightness to a specific value
    Set { pct: u8 },
}
