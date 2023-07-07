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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Bat(PathBuf);

impl Bat {
    async fn read(&self, file_name: impl AsRef<str>) -> Result<String> {
        Ok(read_to_string(self.0.join(file_name.as_ref())).await?)
    }

    async fn read_usize(&self, file_name: impl AsRef<str>) -> Result<usize> {
        Ok(self.read(file_name).await?.trim().parse::<usize>()?)
    }

    fn name(&self) -> Result<String> {
        match self.0.file_name() {
            Some(name) => Ok(name.to_string_lossy().into_owned()),
            None => Err(format!("failed to parse file name from: {}", self.0.display()).into()),
        }
    }

    async fn get_state(&self) -> Result<BatState> {
        Ok(BatState::from_str(self.read("status").await?.trim())?)
    }

    // NOTE: there is also `/capacity` which returns an integer percentage
    async fn percent(&self) -> Result<f32> {
        let (charge_now, charge_full) = try_join!(
            self.read_usize("charge_now"),
            self.read_usize("charge_full"),
        )?;
        Ok((charge_now as f32) / (charge_full as f32) * 100.0)
    }

    async fn watts_now(&self) -> Result<f64> {
        let (current_pico, voltage_pico) = try_join!(
            self.read_usize("current_now"),
            self.read_usize("voltage_now"),
        )?;
        Ok((current_pico as f64) * (voltage_pico as f64) / 1_000_000_000_000.0)
    }

    async fn format(&self, theme: &Theme, show_watts: bool) -> Result<I3Item> {
        let (charge, state) = match try_join!(self.percent(), self.get_state()) {
            Ok((charge, state)) => (charge, state),
            // Return unknown state: the files in sysfs aren't present at times, such as when connecting
            // ac adapters, etc. In these scenarios we just end early here without an error and let the
            // item retry on the next interval/acpi event.
            Err(e) => {
                log::warn!("failed to read battery {}: {}", self.0.display(), e);
                return Ok(I3Item::new("???").color(theme.red));
            }
        };

        let (charge_icon, charge_fg, urgent) = match charge as u32 {
            0..=15 => {
                let urgent = !matches!(state, BatState::Charging | BatState::NotCharging);
                ("", Some(theme.red), urgent)
            }
            16..=25 => ("", Some(theme.orange), false),
            26..=50 => ("", Some(theme.yellow), false),
            51..=75 => ("", None, false),
            76..=u32::MAX => ("", Some(theme.green), false),
        };

        let (state_icon, state_fg) = state.get_color(theme);
        let icon = state_icon.unwrap_or(charge_icon);
        let fg = state_fg.or(charge_fg);

        let item = if show_watts {
            let watts = self.watts_now().await?;
            I3Item::new(format!("{:.2} W", watts)).short_text(format!("{:.0}", watts))
        } else {
            let name = self.name()?;
            let name = if name == "BAT0" {
                icon
            } else {
                name.as_str().into()
            };
            I3Item::new(format!("{}  {:.0}%", name, charge)).short_text(format!("{:.0}%", charge))
        };

        Ok(match (urgent, fg) {
            (true, _) => item.urgent(true),
            (false, Some(fg)) => item.color(fg),
            (false, None) => item,
        })
    }

    async fn find_all() -> Result<Vec<Bat>> {
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

#[async_trait(?Send)]
impl BarItem for Battery {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let batteries = match self.batteries.clone() {
            Some(inner) => inner,
            None => Bat::find_all().await?,
        };

        let mut show_watts = false;
        let mut p = Paginator::new();
        if batteries.len() == 0 {
            bail!("no batteries found");
        } else {
            p.set_len(batteries.len())?;
        }

        let dbus = dbus_connection(BusType::Session).await?;
        let notifications = NotificationsProxy::new(&dbus).await?;
        let mut on_acpi_event = battery_acpi_events().await?;
        loop {
            let theme = &ctx.config.theme;

            let item = batteries[p.idx()].format(theme, show_watts).await?;
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

                if result.is_err() {
                    // SAFETY: we just checked with `.is_err()`
                    break result.unwrap_err();
                }
            }
        };

        log::error!("unexpected failure of battery acpi event stream: {}", err);
    });

    Ok(rx)
}
