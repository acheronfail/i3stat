use std::error::Error;
use std::path::PathBuf;

use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;
use serde_derive::{Deserialize, Serialize};

use crate::bar_items::*;
use crate::context::BarItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Common {
    pub signal: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Item {
    Battery {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Battery,
    },
    Cpu {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Cpu,
    },
    Disk {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Disk,
    },
    Dunst {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Dunst,
    },
    Kbd {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Kbd,
    },
    Mem {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Mem,
    },
    NetUsage {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: NetUsage,
    },
    Nic {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Nic,
    },
    Pulse {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Pulse,
    },
    Script {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Script,
    },
    Sensors {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Sensors,
    },
    Time {
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        inner: Time,
    },
}

impl Item {
    pub fn to_bar_item(&self) -> Box<dyn BarItem> {
        match self {
            Item::Battery { inner, .. } => Box::new(inner.clone()),
            Item::Cpu { inner, .. } => Box::new(inner.clone()),
            Item::Disk { inner, .. } => Box::new(inner.clone()),
            Item::Dunst { inner, .. } => Box::new(inner.clone()),
            Item::Kbd { inner, .. } => Box::new(inner.clone()),
            Item::Mem { inner, .. } => Box::new(inner.clone()),
            Item::NetUsage { inner, .. } => Box::new(inner.clone()),
            Item::Nic { inner, .. } => Box::new(inner.clone()),
            Item::Pulse { inner, .. } => Box::new(inner.clone()),
            Item::Script { inner, .. } => Box::new(inner.clone()),
            Item::Sensors { inner, .. } => Box::new(inner.clone()),
            Item::Time { inner, .. } => Box::new(inner.clone()),
        }
    }

    pub fn common(&self) -> &Common {
        match self {
            Item::Battery { common, .. } => common,
            Item::Cpu { common, .. } => common,
            Item::Disk { common, .. } => common,
            Item::Dunst { common, .. } => common,
            Item::Kbd { common, .. } => common,
            Item::Mem { common, .. } => common,
            Item::NetUsage { common, .. } => common,
            Item::Nic { common, .. } => common,
            Item::Pulse { common, .. } => common,
            Item::Script { common, .. } => common,
            Item::Sensors { common, .. } => common,
            Item::Time { common, .. } => common,
        }
    }

    // TODO: can I use serde's internal "tag" here rather than building it manually here?
    pub fn tag(&self) -> &'static str {
        match self {
            Item::Battery { .. } => "battery",
            Item::Cpu { .. } => "cpu",
            Item::Disk { .. } => "disk",
            Item::Dunst { .. } => "dunst",
            Item::Kbd { .. } => "kbd",
            Item::Mem { .. } => "mem",
            Item::NetUsage { .. } => "net_usage",
            Item::Nic { .. } => "nic",
            Item::Pulse { .. } => "pulse",
            Item::Script { .. } => "script",
            Item::Sensors { .. } => "sensors",
            Item::Time { .. } => "time",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub items: Vec<Item>,
}

pub async fn read(config_path: Option<PathBuf>) -> Result<AppConfig, Box<dyn Error>> {
    let path = config_path
        .map(|p| p.with_extension(""))
        .or_else(|| dirs::config_dir().map(|d| d.join("staturs/config")))
        .ok_or_else(|| "failed to find config dir")?;

    // TODO: document this order in help text
    let c = Figment::new()
        .merge(Toml::file(path.with_extension("toml")))
        .merge(Yaml::file(path.with_extension("yaml")))
        .merge(Json::file(path.with_extension("json")))
        .extract::<AppConfig>()?;

    Ok(c)
}
