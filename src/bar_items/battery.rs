use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use futures::future;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use tokio::fs::{self, read_to_string};

use crate::context::{BarItem, Context};
use crate::format::fraction;
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::theme::Theme;
use crate::BarEvent;

enum BatState {
    Unknown,
    Charging,
    Discharging,
    NotCharging,
    Full,
}

impl BatState {
    fn get_color(&self, theme: &Theme) -> Option<HexColor> {
        match self {
            Self::Full => Some(theme.special),
            Self::Charging => Some(theme.accent1),
            _ => None,
        }
    }
}

impl FromStr for BatState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power
        match s {
            "Unknown" => Ok(Self::Unknown),
            "Charging" => Ok(Self::Charging),
            "Discharging" => Ok(Self::Discharging),
            "Not charging" => Ok(Self::NotCharging),
            "Full" => Ok(Self::Full),
            s => Err(format!("Unknown battery state: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Bat(PathBuf);

impl Bat {
    fn name(&self) -> String {
        self.0.file_name().unwrap().to_string_lossy().into_owned()
    }

    async fn get_state(&self) -> Result<BatState, Box<dyn Error>> {
        Ok(BatState::from_str(
            read_to_string(self.0.join("status")).await?.trim(),
        )?)
    }

    async fn get_charge(&self) -> Result<f32, Box<dyn Error>> {
        macro_rules! get_usize {
            ($x: expr) => {
                read_to_string(self.0.join($x))
                    .await?
                    .trim()
                    .parse::<usize>()? as f32
            };
        }

        Ok(get_usize!("charge_now") / get_usize!("charge_full") * 100.0)
    }

    async fn find_all() -> Result<Vec<Bat>, Box<dyn Error>> {
        let battery_dir = PathBuf::from("/sys/class/power_supply");
        let mut entries = fs::read_dir(&battery_dir).await?;

        let mut batteries = vec![];
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_symlink() {
                let path = entry.path();
                if fs::try_exists(path.join("charge_now")).await? {
                    batteries.push(Bat(path));
                }
            }
        }

        Ok(batteries)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Battery {
    #[serde(with = "humantime_serde")]
    interval: Duration,
    batteries: Option<Vec<Bat>>,
}

impl Battery {
    fn format(theme: &Theme, name: &String, pct: f32, state: BatState) -> (String, String) {
        let fg = state.get_color(theme);
        let (icon, fg) = match pct as u32 {
            0..=15 => (" ", fg.or(Some(theme.error))),
            16..=25 => (" ", fg.or(Some(theme.danger))),
            26..=50 => (" ", fg.or(Some(theme.warning))),
            51..=75 => (" ", fg.or(None)),
            76..=u32::MAX => (" ", fg.or(Some(theme.success))),
        };

        let name = if name == "BAT0" { icon } else { name.as_str() };
        let fg = fg
            .map(|c| format!(r#" foreground="{}""#, c))
            .unwrap_or("".into());
        (
            format!("<span{}>{} {:.0}%</span>", fg, name, pct),
            format!("<span{}>{:.0}%</span>", fg, pct),
        )
    }

    async fn get(bat: &Bat) -> Result<(String, f32, BatState), Box<dyn Error>> {
        let (charge, state) = future::join(bat.get_charge(), bat.get_state()).await;
        Ok((bat.name(), charge?, state?))
    }
}

#[async_trait(?Send)]
impl BarItem for Battery {
    // TODO: investigate waiting on bat/status FD for state changes?
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        let batteries = match self.batteries {
            Some(inner) => inner,
            None => Bat::find_all().await?,
        };

        let mut idx = 0;
        let len = batteries.len();
        loop {
            idx = idx % len;

            let (name, pct, state) = Self::get(&batteries[idx]).await?;
            let (full, short) = Self::format(&ctx.theme, &name, pct, state);
            let full = format!("{}{}", full, fraction(&ctx.theme, idx + 1, len));

            let item = I3Item::new(full)
                .short_text(short)
                .name("bat")
                .markup(I3Markup::Pango);

            ctx.update_item(item).await?;

            // cycle though batteries
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
