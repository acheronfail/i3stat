use std::error::Error;
use std::path::PathBuf;

use config::Config;
use serde_derive::{Deserialize, Serialize};

use crate::bar_items::*;
use crate::context::BarItem;

// TODO: config included in each type of item
// TODO: signal mappings for blocks (common config for each?)
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Item {
    Battery,
    Cpu,
    Disk,
    Dunst,
    Kbd,
    Mem,
    NetUsage,
    Nic,
    Pulse,
    Script {
        #[serde(flatten)]
        inner: Script,
    },
    Sensors,
    Time,
}

impl Item {
    pub fn to_bar_item(self) -> Box<dyn BarItem> {
        match self {
            Item::Battery => Box::new(Battery::default()),
            Item::Cpu => Box::new(Cpu::default()),
            Item::Disk => Box::new(Disk::default()),
            Item::Dunst => Box::new(Dunst::default()),
            Item::Kbd => Box::new(Kbd::default()),
            Item::Mem => Box::new(Mem::default()),
            Item::NetUsage => Box::new(NetUsage::default()),
            Item::Nic => Box::new(Nic::default()),
            Item::Pulse => Box::new(Pulse::default()),
            Item::Script { inner } => Box::new(inner),
            Item::Sensors => Box::new(Sensors::default()),
            Item::Time => Box::new(Time::default()),
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
    Ok(c.try_deserialize()
        .map_err(|e| format!("Failed to parse config: {}", e))?)
}
