use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use futures::future;
use futures_util::try_join;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use tokio::fs::{self, read_to_string};

use crate::context::{BarItem, Context};
use crate::format::fraction;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

enum BatState {
    Unknown,
    Charging,
    Discharging,
    NotCharging,
    Full,
}

impl BatState {
    fn get_color(&self, theme: &Theme) -> (Option<&str>, Option<HexColor>) {
        match self {
            Self::Full => (None, Some(theme.special)),
            Self::Charging => (Some("󰚥"), Some(theme.accent1)),
            _ => (None, None),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Bat(PathBuf);

impl Bat {
    async fn read(&self, file_name: impl AsRef<str>) -> Result<String, Box<dyn Error>> {
        Ok(read_to_string(self.0.join(file_name.as_ref())).await?)
    }

    async fn read_usize(&self, file_name: impl AsRef<str>) -> Result<usize, Box<dyn Error>> {
        Ok(self.read(file_name).await?.trim().parse::<usize>()?)
    }

    fn name(&self) -> Result<String, Box<dyn Error>> {
        match self.0.file_name() {
            Some(name) => Ok(name.to_string_lossy().into_owned()),
            None => Err(format!("failed to parse file name from: {}", self.0.display()).into()),
        }
    }

    async fn get_state(&self) -> Result<BatState, Box<dyn Error>> {
        Ok(BatState::from_str(self.read("status").await?.trim())?)
    }

    async fn get_charge(&self) -> Result<f32, Box<dyn Error>> {
        let (charge_now, charge_full) = try_join!(
            self.read_usize("charge_now"),
            self.read_usize("charge_full"),
        )?;
        Ok((charge_now as f32) / (charge_full as f32) * 100.0)
    }

    // TODO: potentially use this at some stage?
    #[allow(unused)]
    async fn current_watts(&self) -> Result<f64, Box<dyn Error>> {
        let (current_pico, voltage_pico) = try_join!(
            self.read_usize("current_now"),
            self.read_usize("voltage_now"),
        )?;
        Ok((current_pico as f64) * (voltage_pico as f64) / 1_000_000_000_000.0)
    }

    async fn format(&self, theme: &Theme) -> Result<(String, String), Box<dyn Error>> {
        let (charge, state) = future::join(self.get_charge(), self.get_state()).await;
        let name = self.name()?;
        let charge = charge?;
        let state = state?;

        let (icon, fg) = state.get_color(theme);
        let (icon, fg) = match charge as u32 {
            0..=15 => (icon.unwrap_or(" "), fg.or(Some(theme.error))),
            16..=25 => (icon.unwrap_or(" "), fg.or(Some(theme.danger))),
            26..=50 => (icon.unwrap_or(" "), fg.or(Some(theme.warning))),
            51..=75 => (icon.unwrap_or(" "), fg.or(None)),
            76..=u32::MAX => (icon.unwrap_or(" "), fg.or(Some(theme.success))),
        };

        let name = if name == "BAT0" { icon } else { name.as_str() };
        let fg = fg
            .map(|c| format!(r#" foreground="{}""#, c))
            .unwrap_or("".into());

        Ok((
            format!("<span{}>{} {:.0}%</span>", fg, name, charge),
            format!("<span{}>{:.0}%</span>", fg, charge),
        ))
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Battery {
    #[serde(with = "crate::human_time")]
    interval: Duration,
    batteries: Option<Vec<Bat>>,
}

#[async_trait(?Send)]
impl BarItem for Battery {
    // TODO: investigate waiting on bat/status FD for state changes?
    // TODO: watts?
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        let batteries = match self.batteries {
            Some(inner) => inner,
            None => Bat::find_all().await?,
        };

        let mut idx = 0;
        let len = batteries.len();
        loop {
            idx = idx % len;

            let (full, short) = &batteries[idx].format(&ctx.theme).await?;
            let full = format!("{}{}", full, fraction(&ctx.theme, idx + 1, len));

            let item = I3Item::new(full)
                .short_text(short)
                .name("bat")
                .markup(I3Markup::Pango);

            ctx.update_item(item).await?;

            // cycle though batteries
            ctx.delay_with_event_handler(self.interval, |event| {
                Context::paginate(&event, len, &mut idx);
                async {}
            })
            .await;
        }
    }
}
