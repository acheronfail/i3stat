use std::fmt::{Debug, Display};
use std::ops::Add;
use std::rc::Rc;

use clap::ValueEnum;
use libpulse_binding::context::introspect::{SinkInfo, SinkPortInfo, SourceInfo, SourcePortInfo};
use libpulse_binding::def::{DevicePortType, PortAvailable};
use libpulse_binding::volume::{ChannelVolumes, Volume};
use serde_derive::{Deserialize, Serialize};

use crate::i3::I3Item;
use crate::theme::Theme;

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Object {
    Source,
    Sink,
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = <Object as Into<std::rc::Rc<str>>>::into(*self);
        f.write_str(&s)
    }
}

impl From<Object> for Rc<str> {
    fn from(value: Object) -> Self {
        match value {
            Object::Sink => "sink".into(),
            Object::Source => "source".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vol {
    Incr(u32),
    Decr(u32),
    Set(u32),
}

impl Display for Vol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Vol::Incr(amount) => format!("+{}%", amount),
            Vol::Decr(amount) => format!("-{}%", amount),
            Vol::Set(value) => format!("={}%", value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub name: Rc<str>,
    pub description: Rc<str>,
    pub available: PortAvailable,
    pub port_type: DevicePortType,
}

impl Port {
    pub fn available(&self) -> bool {
        !matches!(self.available, PortAvailable::No)
    }
}

macro_rules! impl_port_from {
    ($ty:ty) => {
        impl<'a> From<&'a $ty> for Port {
            fn from(value: &'a $ty) -> Self {
                Port {
                    name: value.name.as_deref().unwrap_or("").into(),
                    description: value.description.as_deref().unwrap_or("").into(),
                    available: value.available,
                    port_type: value.r#type,
                }
            }
        }
    };
}

impl_port_from!(SinkPortInfo<'a>);
impl_port_from!(SourcePortInfo<'a>);

/// Information about a `Sink` or a `Source` (input or output)
#[derive(Debug, Clone)]
pub struct InOut {
    pub index: u32,
    pub name: Rc<str>,
    pub volume: ChannelVolumes,
    pub mute: bool,
    pub ports: Rc<[Port]>,
    pub active_port: Option<Port>,
    pub is_source_monitor: bool,
}

impl<'a> From<&'a SinkInfo<'a>> for InOut {
    fn from(value: &'a SinkInfo<'a>) -> Self {
        InOut {
            index: value.index,
            name: value.name.as_deref().unwrap_or("").into(),
            volume: value.volume,
            mute: value.mute,
            ports: value.ports.iter().map(Port::from).collect(),
            active_port: value.active_port.as_ref().map(|p| Port::from(p.as_ref())),
            is_source_monitor: false,
        }
    }
}

impl<'a> From<&'a SourceInfo<'a>> for InOut {
    fn from(value: &'a SourceInfo<'a>) -> Self {
        InOut {
            index: value.index,
            name: value.name.as_deref().unwrap_or("").into(),
            volume: value.volume,
            mute: value.mute,
            ports: value.ports.iter().map(Port::from).collect(),
            active_port: value.active_port.as_ref().map(|p| Port::from(p.as_ref())),
            is_source_monitor: value.monitor_of_sink.is_some(),
        }
    }
}

impl InOut {
    pub fn volume_pct(&self) -> u32 {
        let normal = Volume::NORMAL.0;
        (self.volume.max().0 * 100 + normal / 2) / normal
    }

    pub fn port_symbol(&self) -> Option<&str> {
        if self.is_source_monitor {
            return Some("󱡫 ");
        }

        match &self.active_port {
            Some(port) => match port.port_type {
                DevicePortType::Aux => Some("󱡬 "),
                DevicePortType::Bluetooth => Some("󰂰 "),
                DevicePortType::Car => Some("󰄋 "),
                DevicePortType::Earpiece => Some("󰟅 "),
                DevicePortType::HDMI => Some("󰡁 "),
                DevicePortType::Headphones => Some("󰋋 "),
                DevicePortType::Headset => Some("󰋎 "),
                DevicePortType::HiFi => Some("󰓃 "),
                DevicePortType::Mic => Some("󰍬 "),
                DevicePortType::Network => Some("󰛳 "),
                DevicePortType::Radio => Some("󰐹 "),
                DevicePortType::TV => Some(" "),
                _ => None,
            },
            None => None,
        }
    }

    fn current_port_idx(&self) -> usize {
        self.active_port.as_ref().map_or(0, |active| {
            match self.ports.iter().position(|p| p == active) {
                Some(idx) => idx,
                None => {
                    log::warn!(
                        "failed to find active port: object={self:?}, active_port={active:?}",
                    );

                    // default to 0
                    0
                }
            }
        })
    }

    pub fn first_port(&self) -> Option<&Port> {
        self.ports.iter().find(|p| p.available())
    }

    pub fn next_port(&self, dir: Dir) -> Option<&Port> {
        let current_idx = self.current_port_idx();
        match dir {
            Dir::Next => self.ports[current_idx + 1..].iter().find(|p| p.available()),
            Dir::Prev => self.ports[..current_idx]
                .iter()
                .rev()
                .find(|p| p.available()),
        }
    }

    pub fn notify_volume_mute(&self) -> Command {
        Command::NotifyVolume {
            name: self.name.clone(),
            volume: self.volume_pct(),
            mute: self.mute,
        }
    }

    pub fn notify_new(&self, r#type: &'static str) -> Command {
        Command::NotifyNewSourceSink {
            name: self.name.clone(),
            what: r#type.into(),
        }
    }

    pub fn format(&self, what: Object, theme: &Theme) -> String {
        format!(
            r#"<span foreground="{}">{} {}%</span>"#,
            (if self.mute { theme.dim } else { theme.fg }).display_rgb(),
            self.port_symbol().unwrap_or(match (what, self.mute) {
                (Object::Sink, false) => "",
                (Object::Sink, true) => "",
                (Object::Source, false) => "󰍬",
                (Object::Source, true) => "󰍭",
            }),
            self.volume_pct(),
        )
    }
}

pub enum Command {
    UpdateItem(Box<dyn FnOnce(&Theme) -> I3Item>),
    NotifyVolume {
        name: Rc<str>,
        volume: u32,
        mute: bool,
    },
    NotifyNewSourceSink {
        name: Rc<str>,
        what: Rc<str>,
    },
    NotifyDefaultsChange {
        name: Rc<str>,
        what: Rc<str>,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationSetting {
    /// No notifications are sent (the default)
    #[default]
    None,
    /// When volumes are changed
    VolumeMute,
    /// When a source or sink is added
    NewSourceSink,
    /// When the default source or sink has changed
    DefaultsChange,
    /// All notifications
    All,
}

impl NotificationSetting {
    pub fn should_notify(&self, ask: Self) -> bool {
        match self {
            NotificationSetting::All => true,
            other => *other == ask,
        }
    }
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Dir {
    Prev,
    Next,
}

impl Dir {
    /// Returns `None` if
    /// * `items` was empty
    /// * `f` excludes all items
    pub fn cycle<'b, T, F>(&self, start: usize, items: &'b [T], f: F) -> Option<&'b T>
    where
        F: Fn(&&T) -> bool,
    {
        let limit = items.len() * 2;
        match self {
            Dir::Next => items.iter().cycle().skip(start + 1).take(limit).find(f),
            Dir::Prev => items
                .iter()
                .rev()
                .cycle()
                .skip(items.len() - start)
                .take(limit)
                .find(f),
        }
    }
}

impl Add<usize> for Dir {
    type Output = usize;

    fn add(self, rhs: usize) -> Self::Output {
        match self {
            Dir::Prev => rhs - 1,
            Dir::Next => rhs + 1,
        }
    }
}

impl Add<Dir> for usize {
    type Output = usize;

    fn add(self, rhs: Dir) -> Self::Output {
        match rhs {
            Dir::Prev => self - 1,
            Dir::Next => self + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /**
     * Port tests
     */

    macro_rules! port {
        ($name:expr, $available:expr, $type:expr) => {
            Port {
                name: $name.into(),
                description: $name.into(),
                available: $available,
                port_type: $type,
            }
        };
    }

    macro_rules! obj {
        ($index:literal, $name:expr, $ports:expr, active = $active:expr) => {
            InOut {
                index: $index,
                name: $name.into(),
                volume: ChannelVolumes::default(),
                mute: false,
                ports: $ports.clone().into(),
                active_port: $ports.get($active).cloned(),
                is_source_monitor: false,
            }
        };
    }

    #[test]
    fn port_cycling_single_port() {
        let ports = vec![port!("a", PortAvailable::Yes, DevicePortType::Speaker)];
        let obj = obj!(0, "one", ports, active = 0);
        assert_eq!(obj.current_port_idx(), 0);
        assert_eq!(obj.first_port(), Some(&ports[0]));
        assert_eq!(obj.next_port(Dir::Next), None);
        assert_eq!(obj.next_port(Dir::Prev), None);
    }

    #[test]
    fn port_cycling_two_ports() {
        let ports = vec![
            port!("a", PortAvailable::Yes, DevicePortType::Speaker),
            port!("b", PortAvailable::Yes, DevicePortType::Speaker),
        ];

        let obj = obj!(0, "one", ports, active = 0);
        assert_eq!(obj.current_port_idx(), 0);
        assert_eq!(obj.first_port(), Some(&ports[0]));

        let obj = obj!(0, "one", ports, active = 1);
        assert_eq!(obj.next_port(Dir::Next), None);
        assert_eq!(obj.next_port(Dir::Prev), Some(&ports[0]));
    }

    #[test]
    fn port_cycling_skip_unavailable() {
        let ports = vec![
            port!("a", PortAvailable::Yes, DevicePortType::Speaker),
            port!("b", PortAvailable::No, DevicePortType::Speaker),
            port!("c", PortAvailable::Unknown, DevicePortType::Speaker),
        ];

        let obj = obj!(0, "one", ports, active = 0);
        assert_eq!(obj.next_port(Dir::Next), Some(&ports[2]));
        assert_eq!(obj.next_port(Dir::Prev), None);
        let obj = obj!(0, "one", ports, active = 2);
        assert_eq!(obj.next_port(Dir::Next), None);
        assert_eq!(obj.next_port(Dir::Prev), Some(&ports[0]));
    }

    /**
     * Dir tests
     */

    #[test]
    fn dir_cycle_none() {
        assert_eq!(Dir::Next.cycle(0, &[], |()| true), None);
        assert_eq!(Dir::Prev.cycle(0, &[], |()| true), None);
    }

    #[test]
    fn dir_cycle_one() {
        assert_eq!(Dir::Next.cycle(0, &[1], |_| true), Some(&1));
        assert_eq!(Dir::Prev.cycle(0, &[1], |_| true), Some(&1));
    }

    #[test]
    fn dir_cycle_one_filter() {
        assert_eq!(Dir::Next.cycle(0, &[1], |_| false), None);
        assert_eq!(Dir::Prev.cycle(0, &[1], |_| false), None);
    }

    #[test]
    fn dir_cycle_many() {
        assert_eq!(Dir::Next.cycle(0, &[1, 2, 3], |_| true), Some(&2));
        assert_eq!(Dir::Next.cycle(1, &[1, 2, 3], |_| true), Some(&3));
        assert_eq!(Dir::Next.cycle(2, &[1, 2, 3], |_| true), Some(&1));

        assert_eq!(Dir::Prev.cycle(0, &[1, 2, 3], |_| true), Some(&3));
        assert_eq!(Dir::Prev.cycle(1, &[1, 2, 3], |_| true), Some(&1));
        assert_eq!(Dir::Prev.cycle(2, &[1, 2, 3], |_| true), Some(&2));
    }

    #[test]
    fn dir_cycle_many_filter() {
        assert_eq!(Dir::Next.cycle(0, &[1, 2, 3], |x| **x != 2), Some(&3));
        assert_eq!(Dir::Next.cycle(1, &[1, 2, 3], |x| **x != 2), Some(&3));
        assert_eq!(Dir::Next.cycle(2, &[1, 2, 3], |x| **x != 2), Some(&1));

        assert_eq!(Dir::Prev.cycle(0, &[1, 2, 3], |x| **x != 2), Some(&3));
        assert_eq!(Dir::Prev.cycle(1, &[1, 2, 3], |x| **x != 2), Some(&1));
        assert_eq!(Dir::Prev.cycle(2, &[1, 2, 3], |x| **x != 2), Some(&1));

        assert_eq!(Dir::Next.cycle(0, &[1, 2, 3], |x| **x == 9), None);
        assert_eq!(Dir::Next.cycle(1, &[1, 2, 3], |x| **x == 9), None);
        assert_eq!(Dir::Next.cycle(2, &[1, 2, 3], |x| **x == 9), None);

        assert_eq!(Dir::Prev.cycle(0, &[1, 2, 3], |x| **x == 9), None);
        assert_eq!(Dir::Prev.cycle(1, &[1, 2, 3], |x| **x == 9), None);
        assert_eq!(Dir::Prev.cycle(2, &[1, 2, 3], |x| **x == 9), None);
    }

    #[test]
    fn dir_cycle_start_past_end() {
        assert_eq!(Dir::Next.cycle(999, &[1], |_| true), Some(&1));
    }
}
