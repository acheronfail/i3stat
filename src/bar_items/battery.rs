use std::cell::OnceCell;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use futures::try_join;
use hex_color::HexColor;
use serde_derive::{Deserialize, Serialize};
use tokio::fs::{self, read_to_string};
use tokio::sync::mpsc::Receiver;

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::dbus::notifications::NotificationsProxy;
use crate::dbus::{dbus_connection, BusType};
use crate::error::Result;
use crate::i3::{I3Button, I3Item, I3Markup};
use crate::theme::Theme;
use crate::util::acpi::ffi::AcpiGenericNetlinkEvent;
use crate::util::{netlink_acpi_listen, Paginator};

#[derive(Debug)]
enum BatState {
    Unknown,
    Charging,
    Discharging,
    NotCharging,
    Full,
}

impl BatState {
    fn get_color(&self, theme: &Theme) -> (Option<&'static str>, Option<HexColor>) {
        match self {
            Self::Full => (None, Some(theme.purple)),
            Self::Charging => (Some("󰚥"), Some(theme.blue)),
            _ => (None, None),
        }
    }
}

impl FromStr for BatState {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

#[derive(Debug)]
struct BatInfo {
    name: String,
    charge: f32,
    state: BatState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct Bat {
    /// Battery directory in procfs.
    dir: PathBuf,

    /// Whether `/charge_{now,full}` is used or `/capacity` to check battery levels.
    #[serde(skip)]
    uses_charge: OnceCell<bool>,

    /// Name of the battery.
    #[serde(skip)]
    cached_name: OnceCell<String>,
}

impl Bat {
    pub fn new(dir: PathBuf) -> Bat {
        Bat {
            dir,
            uses_charge: OnceCell::new(),
            cached_name: OnceCell::new(),
        }
    }

    fn name(&self) -> Result<&String> {
        match self.cached_name.get() {
            Some(cached) => Ok(cached),
            None => match self.dir.file_name() {
                Some(name) => match self.cached_name.set(name.to_string_lossy().into_owned()) {
                    Ok(()) => self.name(),
                    Err(_) => bail!("failed to set name cache"),
                },
                None => bail!("failed to parse file name from: {}", self.dir.display()),
            },
        }
    }

    fn uses_charge(&self) -> bool {
        *self.uses_charge.get_or_init(|| {
            self.dir.join("charge_now").exists() && self.dir.join("charge_full").exists()
        })
    }

    async fn read(&self, file_name: impl AsRef<str>) -> Result<String> {
        Ok(read_to_string(self.dir.join(file_name.as_ref())).await?)
    }

    async fn read_usize(&self, file_name: impl AsRef<str>) -> Result<usize> {
        Ok(self.read(file_name).await?.trim().parse::<usize>()?)
    }

    pub async fn get_state(&self) -> Result<BatState> {
        Ok(BatState::from_str(self.read("status").await?.trim())?)
    }

    pub async fn percent(&self) -> Result<f32> {
        match self.uses_charge() {
            false => Ok(self.read_usize("capacity").await? as f32),
            true => {
                let (charge_now, charge_full) = try_join!(
                    self.read_usize("charge_now"),
                    self.read_usize("charge_full"),
                )?;
                Ok((charge_now as f32) / (charge_full as f32) * 100.0)
            }
        }
    }

    pub async fn watts_now(&self) -> Result<f64> {
        let (current_pico, voltage_pico) = try_join!(
            self.read_usize("current_now"),
            self.read_usize("voltage_now"),
        )?;
        Ok((current_pico as f64) * (voltage_pico as f64) / 1_000_000_000_000.0)
    }

    pub async fn get_info(&self) -> Result<BatInfo> {
        let name = self.name()?.to_owned();
        try_join!(self.percent(), self.get_state()).map(|(charge, state)| BatInfo {
            name,
            charge,
            state,
        })
    }

    pub async fn find_all() -> Result<Vec<Bat>> {
        let battery_dir = PathBuf::from("/sys/class/power_supply");
        let mut entries = fs::read_dir(&battery_dir).await?;

        let mut batteries = vec![];
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_symlink() {
                let file = entry.path();
                let b = fs::try_exists(file.join("capacity")).await;
                let a = fs::try_exists(file.join("charge_now")).await;

                match (a, b) {
                    (Ok(true), _) | (_, Ok(true)) => {
                        log::debug!("autodetected battery: {}", &file.display());
                        batteries.push(Bat::new(file));
                    }
                    _ => {}
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
    #[serde(default)]
    notify_percentage: Option<u8>,
}

impl Battery {
    fn detail(theme: &Theme, info: &BatInfo) -> (&'static str, Option<HexColor>, bool) {
        let (charge_icon, charge_fg, urgent) = match info.charge as u32 {
            0..=15 => {
                let urgent = !matches!(info.state, BatState::Charging | BatState::NotCharging);
                ("", Some(theme.red), urgent)
            }
            16..=25 => ("", Some(theme.orange), false),
            26..=50 => ("", Some(theme.yellow), false),
            51..=75 => ("", None, false),
            76..=u32::MAX => ("", Some(theme.green), false),
        };

        let (state_icon, state_fg) = info.state.get_color(theme);
        let icon = state_icon.unwrap_or(charge_icon);
        let fg = state_fg.or(charge_fg);

        (icon, fg, urgent)
    }

    fn format_watts(_: &Theme, watts: f64) -> I3Item {
        I3Item::new(format!("{:.2} W", watts)).short_text(format!("{:.0}", watts))
    }

    async fn format(_: &Theme, info: &BatInfo, icon: &str) -> I3Item {
        let name = if info.name == "BAT0" {
            icon
        } else {
            info.name.as_str()
        };
        I3Item::new(format!("{}  {:.0}%", name, info.charge))
            .short_text(format!("{:.0}%", info.charge))
    }
}

#[async_trait(?Send)]
impl BarItem for Battery {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let batteries = match self.batteries.clone() {
            Some(inner) => inner,
            None => Bat::find_all().await?,
        };

        let mut show_watts = false;
        let mut p = Paginator::new();
        if batteries.is_empty() {
            bail!("no batteries found");
        } else {
            p.set_len(batteries.len())?;
        }

        let dbus = dbus_connection(BusType::Session).await?;
        let notifications = NotificationsProxy::new(dbus).await?;
        let mut on_acpi_event = battery_acpi_events().await?;
        let mut sent_critical_notification = false;
        loop {
            let theme = &ctx.config.theme;

            // get info for selected battery
            let bat = &batteries[p.idx()];
            let info = bat.get_info().await?;

            // send critical battery notification if configured
            if let Some(pct) = self.notify_percentage {
                let charge = info.charge as u8;
                if charge <= pct && matches!(info.state, BatState::Discharging) {
                    notifications.battery_critical(charge).await;
                    sent_critical_notification = true;
                } else if sent_critical_notification {
                    notifications.battery_critical_off().await;
                    sent_critical_notification = false;
                }
            }

            // build battery item
            let (icon, fg, urgent) = Self::detail(theme, &info);
            let item = if show_watts {
                Self::format_watts(theme, bat.watts_now().await?)
            } else {
                Self::format(theme, &info, icon).await
            };

            // format item
            let item = match (fg, urgent) {
                (_, true) => item.urgent(true),
                (Some(fg), false) => item.color(fg),
                (None, false) => item,
            };

            // update item
            let full_text = format!("{}{}", item.get_full_text(), p.format(theme));
            let item = item.full_text(full_text).markup(I3Markup::Pango);
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

            tokio::select! {
                // reload block on click (or timeout)
                () = wait_for_click => {},
                // reload block on any ACPI event
                Some(event) = on_acpi_event.recv() => {
                    if let BatteryAcpiEvent::AcAdapterPlugged(plugged_in) = event {
                        if self.notify_on_adapter {
                            let _ = notifications.ac_adapter(plugged_in).await;
                        }
                    }
                },
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum BatteryAcpiEvent {
    Battery,
    AcAdapterPlugged(bool),
}

async fn battery_acpi_events() -> Result<Receiver<BatteryAcpiEvent>> {
    let mut acpi_event = netlink_acpi_listen().await?;
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    tokio::task::spawn_local(async move {
        let err = loop {
            if let Some(ev) = acpi_event.recv().await {
                let result = match ev.device_class.as_str() {
                    // refresh on ac adapter events
                    AcpiGenericNetlinkEvent::DEVICE_CLASS_AC => {
                        tx.send(BatteryAcpiEvent::AcAdapterPlugged(ev.data == 1))
                            .await
                    }
                    // refresh on battery related events
                    AcpiGenericNetlinkEvent::DEVICE_CLASS_BATTERY => {
                        tx.send(BatteryAcpiEvent::Battery).await
                    }
                    // ignore other acpi events
                    _ => continue,
                };

                if let Err(err) = result {
                    break err;
                }
            }
        };

        log::error!("unexpected failure of battery acpi event stream: {}", err);
    });

    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de() {
        let path = "/sys/class/power_supply/BAT0";
        let battery = serde_json::from_str::<Bat>(&format!(r#""{}""#, path)).unwrap();
        assert_eq!(battery.dir, PathBuf::from(path));
    }

    #[test]
    fn name() {
        let battery = Bat::new(PathBuf::from("/sys/class/power_supply/BAT0"));
        assert_eq!(battery.name().unwrap(), "BAT0");
        assert_eq!(battery.name().unwrap(), "BAT0");
    }
}
