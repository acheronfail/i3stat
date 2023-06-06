use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use futures::{future, try_join};
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use tokio::fs::{self, read_to_string};

use crate::context::{BarEvent, BarItem, Context};
use crate::dbus::notifications::NotificationsProxy;
use crate::dbus::{dbus_connection, BusType};
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::ffi::AcpiGenericNetlinkEvent;
use crate::util::{netlink_acpi_listen, Paginator};

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
            Self::Full => (None, Some(theme.purple)),
            Self::Charging => (Some("󰚥"), Some(theme.blue)),
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

    // NOTE: there is also `/capacity` which returns an integer percentage
    async fn percent(&self) -> Result<f32, Box<dyn Error>> {
        let (charge_now, charge_full) = try_join!(
            self.read_usize("charge_now"),
            self.read_usize("charge_full"),
        )?;
        Ok((charge_now as f32) / (charge_full as f32) * 100.0)
    }

    async fn watts_now(&self) -> Result<f64, Box<dyn Error>> {
        let (current_pico, voltage_pico) = try_join!(
            self.read_usize("current_now"),
            self.read_usize("voltage_now"),
        )?;
        Ok((current_pico as f64) * (voltage_pico as f64) / 1_000_000_000_000.0)
    }

    async fn format(
        &self,
        theme: &Theme,
        show_watts: bool,
    ) -> Result<(String, String, Option<HexColor>), Box<dyn Error>> {
        let (charge, state) = future::join(self.percent(), self.get_state()).await;
        let charge = charge?;
        let state = state?;

        let (icon, fg) = state.get_color(theme);
        let (icon, fg) = match charge as u32 {
            0..=15 => (icon.unwrap_or(""), fg.or(Some(theme.red))),
            16..=25 => (icon.unwrap_or(""), fg.or(Some(theme.orange))),
            26..=50 => (icon.unwrap_or(""), fg.or(Some(theme.yellow))),
            51..=75 => (icon.unwrap_or(""), fg.or(None)),
            76..=u32::MAX => (icon.unwrap_or(""), fg.or(Some(theme.green))),
        };

        if show_watts {
            let watts = self.watts_now().await?;
            Ok((format!("{:.2} W", watts), format!("{:.0}", watts), fg))
        } else {
            let name = self.name()?;
            let name = if name == "BAT0" {
                icon
            } else {
                name.as_str().into()
            };
            Ok((
                format!("{}  {:.0}%", name, charge),
                format!("{:.0}%", charge),
                fg,
            ))
        }
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
    #[serde(default)]
    notify_on_adapter: bool,
    // TODO: option to run command(s) at certain percentage(s)
}

// FIXME: sometimes the battery disappears when AC connected and this block exits
//  should be fixed with a retry, or by refreshing `batteries`
#[async_trait(?Send)]
impl BarItem for Battery {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        let batteries = match self.batteries {
            Some(inner) => inner,
            None => Bat::find_all().await?,
        };

        let mut show_watts = false;
        let mut p = Paginator::new();
        if batteries.len() == 0 {
            return Err("no batteries found".into());
        } else {
            p.set_len(batteries.len());
        }

        let dbus = dbus_connection(BusType::Session).await?;
        let notifications = NotificationsProxy::new(&dbus).await?;
        let mut acpi_event = netlink_acpi_listen().await?;
        loop {
            let theme = &ctx.config.theme;
            let (full, short, fg) = batteries[p.idx()].format(theme, show_watts).await?;
            let full = format!("{}{}", full, p.format(theme));

            let mut item = I3Item::new(full).short_text(short).markup(I3Markup::Pango);
            if let Some(color) = fg {
                item = item.color(color);
            }

            ctx.update_item(item).await?;

            // change delay if we're displaying watts
            let delay = if show_watts {
                Duration::from_secs(2)
            } else {
                self.interval
            };

            // cycle though batteries
            let wait_for_click = ctx.delay_with_event_handler(delay, |event| {
                p.update(&event);
                if let BarEvent::Click(click) = event {
                    if click.button == I3Button::Middle {
                        show_watts = !show_watts;
                    }
                }
                async {}
            });

            // update on acpi events
            let wait_for_acpi_event = async {
                loop {
                    if let Some(ev) = acpi_event.recv().await {
                        match ev.device_class.as_str() {
                            // refresh on battery related events
                            AcpiGenericNetlinkEvent::DEVICE_CLASS_AC => break Some(ev.data == 1),
                            AcpiGenericNetlinkEvent::DEVICE_CLASS_BATTERY => break None,
                            // ignore other acpi events
                            _ => continue,
                        }
                    }
                }
            };

            tokio::select! {
                // reload block on click (or timeout)
                () = wait_for_click => {},
                // reload block on any ACPI event
                ac_state = wait_for_acpi_event => {
                    if let Some(plugged_in) = ac_state {
                        if self.notify_on_adapter {
                            let _ = notifications.ac_adapter(plugged_in).await;
                        }
                    }
                },
            }
        }
    }
}
