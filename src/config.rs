use std::error::Error;
use std::path::PathBuf;

use config::Config;
use serde_derive::{Deserialize, Serialize};

use crate::bar_items::*;
use crate::context::BarItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Common {
    pub signal: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub fn to_bar_item(self) -> (Common, Box<dyn BarItem>) {
        match self {
            Item::Battery { inner, common } => (common, Box::new(inner)),
            Item::Cpu { inner, common } => (common, Box::new(inner)),
            Item::Disk { inner, common } => (common, Box::new(inner)),
            Item::Dunst { inner, common } => (common, Box::new(inner)),
            Item::Kbd { inner, common } => (common, Box::new(inner)),
            Item::Mem { inner, common } => (common, Box::new(inner)),
            Item::NetUsage { inner, common } => (common, Box::new(inner)),
            Item::Nic { inner, common } => (common, Box::new(inner)),
            Item::Pulse { inner, common } => (common, Box::new(inner)),
            Item::Script { inner, common } => (common, Box::new(inner)),
            Item::Sensors { inner, common } => (common, Box::new(inner)),
            Item::Time { inner, common } => (common, Box::new(inner)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub items: Vec<Item>,
}

pub async fn read(config_path: Option<PathBuf>) -> Result<AppConfig, Box<dyn Error>> {
    let path = config_path
        .or_else(|| dirs::config_dir().map(|d| d.join("staturs/config")))
        .ok_or_else(|| "Failed to find config")?;

    let c = Config::builder()
        .add_source(config::File::from(path).required(true))
        .build()?;

    // TODO: print a single JSON object to STDOUT to display an error rather than crashing?
    //
    Ok(c.try_deserialize()
        // TODO: more detailed error messages?
        .map_err(|e| format!("Failed to parse config: {}", e))?)
}
