use std::cell::OnceCell;
use std::collections::HashSet;
use std::error::Error;
use std::ffi::OsStr;
use std::path::PathBuf;

use figment::error::Kind;
use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;
use indexmap::IndexMap;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use strum::EnumIter;

use crate::bar_items::*;
use crate::cli::Cli;
use crate::context::BarItem;
use crate::i3::I3Item;
use crate::ipc::get_socket_path;
use crate::theme::Theme;
use crate::util::sort_by_indices;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Optional list of other configuration files to parse.
    /// The paths should be relative to the main configuration file's directory.
    #[serde(default)]
    include: Vec<PathBuf>,
    /// Path to the socket to use for ipc. Useful when having multiple bars to separate their sockets.
    /// The CLI option takes precedence over this.
    socket: Option<PathBuf>,
    /// Specify the colours of the theme
    #[serde(default)]
    pub theme: Theme,
    /// List of the items for the bar
    pub items: Vec<Item>,
    /// The order that the items should be displayed, left to right.
    /// A list of either strings (item "name"s) or numbers (item indices).
    /// If not specified, then they appear in the order defined in the configuration.
    #[serde(rename = "item_order")]
    item_order: Option<Vec<Value>>,

    /// Runtime only cache for name to index item mappings
    #[serde(skip)]
    name_to_idx: OnceCell<IndexMap<String, usize>>,
    /// Runtime only cache for index to name item mappings
    #[serde(skip)]
    idx_to_name: OnceCell<IndexMap<usize, String>>,
}

impl AppConfig {
    pub fn socket(&self) -> PathBuf {
        // SAFETY: when creating instances of `AppConfig` this option is always filled
        self.socket.clone().unwrap()
    }

    pub fn item_idx_to_name(&self) -> &IndexMap<usize, String> {
        self.idx_to_name.get_or_init(|| {
            let mut map = self
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| (idx, item.name().to_owned()))
                .collect::<IndexMap<usize, String>>();

            map.sort_keys();
            map
        })
    }

    fn item_name_to_idx(&self) -> &IndexMap<String, usize> {
        self.name_to_idx.get_or_init(|| {
            let mut map = self
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| (item.name().to_owned(), idx))
                .collect::<IndexMap<String, usize>>();

            map.sort_keys();
            map
        })
    }

    pub async fn read(args: Cli) -> Result<AppConfig, Box<dyn Error>> {
        let cfg_file = args
            .config
            .or_else(|| dirs::config_dir().map(|d| d.join("istat/config")))
            .ok_or_else(|| "failed to find config file")?;

        let cfg_dir = cfg_file
            .parent()
            .ok_or_else(|| "failed to find config dir")?;

        // parse main configuration
        let mut figment = Figment::new()
            .merge(Toml::file(cfg_file.with_extension("toml")))
            .merge(Json::file(cfg_file.with_extension("json")))
            .merge(Yaml::file(cfg_file.with_extension("yaml")))
            .merge(Yaml::file(cfg_file.with_extension("yml")));

        // include other config files (recursively) if any were specified
        let figment = {
            let mut seen_config_files = HashSet::new();
            seen_config_files.insert(cfg_file.clone());
            loop {
                let include_paths = match figment.extract_inner::<Vec<PathBuf>>("include") {
                    // we got some include paths, make them relative to the main config file
                    Ok(paths) => paths
                        .into_iter()
                        .map(|p| cfg_dir.join(p).canonicalize())
                        .collect::<Result<Vec<_>, _>>()?,
                    // ignore if "include" wasn't specified at all
                    Err(e) if matches!(e.kind, Kind::MissingField(_)) => vec![],
                    // some other error occurred
                    Err(e) => bail!(e),
                };

                if include_paths.iter().all(|p| seen_config_files.contains(p)) {
                    break figment;
                }

                for include in include_paths {
                    match include.extension().and_then(OsStr::to_str) {
                        Some("toml") => figment = figment.admerge(Toml::file(&include)),
                        Some("json") => figment = figment.admerge(Json::file(&include)),
                        Some("yaml") | Some("yml") => {
                            figment = figment.admerge(Yaml::file(&include))
                        }
                        Some(e) => bail!("Unsupported file extension: {}", e),
                        None => bail!("No file extension, cannot infer file format"),
                    }

                    seen_config_files.insert(include);
                }
            }
        };

        // extract our application configuration
        let mut cfg = figment.extract::<AppConfig>()?;

        // set socket path
        cfg.socket = Some(match args.socket {
            Some(socket_path) => socket_path,
            None => get_socket_path(cfg.socket.as_ref())?,
        });

        // config validation
        {
            // convert user `item_order` to indices
            let item_order = match cfg.item_order {
                Some(ref user_order) => {
                    if user_order.len() != cfg.items.len() {
                        bail!(
                            "`item_order` must have the same length as `items`; got length={}, but item count={}",
                            user_order.len(),
                            cfg.items.len()
                        );
                    }

                    let idx_to_name = cfg.item_idx_to_name();
                    let name_to_idx = cfg.item_name_to_idx();
                    let mut order = vec![];
                    for value in user_order {
                        let idx = match value {
                            // parse string as item name
                            Value::String(needle) => match name_to_idx.get(needle) {
                                Some(idx) => *idx,
                                None => bail!("no item found with name: {}", needle),
                            },
                            // parse number as item index
                            Value::Number(idx) => match idx.as_u64() {
                                Some(idx) => idx as usize,
                                None => bail!("not a valid index"),
                            },
                            _ => bail!("only names (strings) or indices (numbers) are allowed"),
                        };

                        if order.contains(&idx) {
                            bail!(
                                "duplicate item defined in `item_order`; index={} and name={}",
                                idx,
                                idx_to_name[idx]
                            );
                        }

                        order.push(idx);
                    }

                    order
                }
                None => (0..cfg.items.len()).collect(),
            };

            // reorder the items
            sort_by_indices(&mut cfg.items, item_order);

            // check no duplicate names
            for (i, a) in cfg.items.iter().enumerate().rev() {
                for (j, b) in cfg.items.iter().enumerate() {
                    if i == j {
                        continue;
                    }

                    if let (Some(a), Some(b)) = (&a.common.name, &b.common.name) {
                        if a == b {
                            bail!(
                                "item names must be unique, item[{}] and item[{}] share the same name: {}",
                                i, j, a
                            );
                        }
                    }
                }
            }

            // check no empty powerline config
            if cfg.theme.powerline.len() <= 1 {
                bail!("theme.powerline must contain at least two values");
            }
        }

        Ok(cfg)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Common {
    pub signal: Option<u32>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, EnumIter)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ItemInner {
    Raw(I3Item),
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
            ItemInner::Raw(_) => "raw",
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

    /// A runtime only cache for this item's name
    #[serde(skip)]
    name: OnceCell<String>,
}

impl Item {
    pub fn to_bar_item(&self) -> Box<dyn BarItem> {
        match &self.inner {
            ItemInner::Raw(inner) => Box::new(inner.clone()),
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

    pub fn name(&self) -> &String {
        self.name.get_or_init(|| match self.common.name {
            Some(ref name) => name.to_string(),
            None => self.inner.tag().into(),
        })
    }
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
