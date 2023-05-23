use std::error::Error;
use std::path::PathBuf;

use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;
use serde_derive::{Deserialize, Serialize};
use strum::EnumIter;

use crate::bar_items::*;
use crate::context::BarItem;
use crate::theme::Theme;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// Path to the socket to use for ipc. Useful when having multiple bars to separate their sockets.
    /// The CLI option takes precedence over this.
    pub socket: Option<PathBuf>,
    /// Specify the colours of the theme
    #[serde(default)]
    pub theme: Theme,
    /// List of the items for the bar - ordered left to right.
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Common {
    pub signal: Option<u32>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, EnumIter)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ItemInner {
    Battery(Battery),
    Cpu(Cpu),
    Disk(Disk),
    Dunst(Dunst),
    Kbd(Kbd),
    Krb(Krb),
    Mem(Mem),
    NetUsage(NetUsage),
    Nic(Nic),
    Pulse(Pulse),
    Script(Script),
    Sensors(Sensors),
    Time(Time),
}

impl ItemInner {
    // Can't seem to use serde to access the tags, even though it's automatically derived the tags.
    // For now, we have a test ensuring this is accurate.
    // See: https://github.com/serde-rs/serde/issues/2455
    pub fn tag(&self) -> &'static str {
        match self {
            ItemInner::Battery(_) => "battery",
            ItemInner::Cpu(_) => "cpu",
            ItemInner::Disk(_) => "disk",
            ItemInner::Dunst(_) => "dunst",
            ItemInner::Kbd(_) => "kbd",
            ItemInner::Krb(_) => "krb",
            ItemInner::Mem(_) => "mem",
            ItemInner::NetUsage(_) => "net_usage",
            ItemInner::Nic(_) => "nic",
            ItemInner::Pulse(_) => "pulse",
            ItemInner::Script(_) => "script",
            ItemInner::Sensors(_) => "sensors",
            ItemInner::Time(_) => "time",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(flatten)]
    pub common: Common,
    #[serde(flatten)]
    inner: ItemInner,
}

impl Item {
    pub fn to_bar_item(&self) -> Box<dyn BarItem> {
        match &self.inner {
            ItemInner::Battery(inner) => Box::new(inner.clone()),
            ItemInner::Cpu(inner) => Box::new(inner.clone()),
            ItemInner::Disk(inner) => Box::new(inner.clone()),
            ItemInner::Dunst(inner) => Box::new(inner.clone()),
            ItemInner::Kbd(inner) => Box::new(inner.clone()),
            ItemInner::Krb(inner) => Box::new(inner.clone()),
            ItemInner::Mem(inner) => Box::new(inner.clone()),
            ItemInner::NetUsage(inner) => Box::new(inner.clone()),
            ItemInner::Nic(inner) => Box::new(inner.clone()),
            ItemInner::Pulse(inner) => Box::new(inner.clone()),
            ItemInner::Script(inner) => Box::new(inner.clone()),
            ItemInner::Sensors(inner) => Box::new(inner.clone()),
            ItemInner::Time(inner) => Box::new(inner.clone()),
        }
    }

    pub fn tag(&self) -> &'static str {
        self.inner.tag()
    }
}

pub async fn read(config_path: Option<PathBuf>) -> Result<AppConfig, Box<dyn Error>> {
    let path = config_path
        .map(|p| p.with_extension(""))
        .or_else(|| dirs::config_dir().map(|d| d.join("istat/config")))
        .ok_or_else(|| "failed to find config dir")?;

    // TODO: document this order in help text
    let c = Figment::new()
        .merge(Toml::file(path.with_extension("toml")))
        .merge(Yaml::file(path.with_extension("yaml")))
        .merge(Json::file(path.with_extension("json")))
        .extract::<AppConfig>()?;

    // config validation
    {
        // check no duplicate names
        for (i, a) in c.items.iter().enumerate().rev() {
            for (j, b) in c.items.iter().enumerate() {
                if i == j {
                    continue;
                }

                if let (Some(a), Some(b)) = (&a.common.name, &b.common.name) {
                    if a == b {
                        return Err(format!(
                                "item names must be unique, item[{}] and item[{}] share the same name: {}",
                                i, j, a
                            )
                            .into());
                    }
                }
            }
        }

        // check no empty powerline config
        if c.theme.powerline.len() <= 1 {
            return Err("theme.powerline must contain at least two values".into());
        }
    }

    Ok(c)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn item_tags() {
        let assert_tag = |item: &ItemInner| {
            let v = json!(item);
            let serialised_tag = v.get("type").unwrap();
            let computed_tag = item.tag();
            assert_eq!(
                serialised_tag, computed_tag,
                "item tags did not match, expected {} got {}",
                serialised_tag, computed_tag
            );
        };

        // iterate over all enums and assert tags
        for variant in ItemInner::iter() {
            assert_tag(&variant);
        }
    }
}
