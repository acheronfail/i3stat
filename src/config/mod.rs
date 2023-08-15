mod item;
mod parse;

use std::cell::OnceCell;
use std::path::PathBuf;

use indexmap::IndexMap;
use serde_derive::{Deserialize, Serialize};

use crate::cli::Cli;
use crate::config::item::Item;
use crate::error::Result;
use crate::ipc::get_socket_path;
use crate::theme::Theme;
use crate::util::sort_by_indices;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Optional list of paths of other configuration files to include.
    /// The paths can be absolute or relative to the main configuration file's directory.
    /// Shell syntax is also expanded (see **wordexp(3)** for details).
    #[serde(default)]
    include: Vec<String>,

    /// Specify the colours of the theme
    #[serde(default)]
    pub theme: Theme,

    /// List of the items for the bar
    pub items: Vec<Item>,

    /// Path to the socket to use for ipc. Useful when having multiple bars to separate their sockets.
    /// The CLI option takes precedence over this.
    #[serde(rename = "socket")]
    socket: Option<PathBuf>,
    /// Runtime only cache for the resolved socket path.

    /// Runtime only cache for index to name item mappings
    #[serde(skip)]
    idx_to_name: OnceCell<IndexMap<usize, String>>,
}

impl AppConfig {
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

    // NOTE: this workaround exists due to a limitation in serde
    // see: https://github.com/serde-rs/serde/issues/2249
    pub fn socket(&self) -> PathBuf {
        // SAFETY: when creating instances of `AppConfig` this option is always filled
        self.socket.clone().unwrap()
    }

    /// Sort the items by reading the index defined in the configuration.
    fn sort(mut items: &mut [Item]) {
        let len = items.len();
        let max = len.saturating_sub(1);
        let mut item_order = (0..len).collect::<Vec<usize>>();
        for (current_idx, item) in items.iter().enumerate() {
            if let Some(target_idx) = item.common.index {
                let idx = item_order.remove(current_idx);
                item_order.insert(target_idx.clamp(0, max), idx);
            }
        }

        sort_by_indices(&mut items, item_order);
    }

    /// Ensure configuration of item names have no duplicates.
    fn validate_names(items: &[Item]) -> Result<()> {
        for (i, a) in items.iter().enumerate().rev() {
            for (j, b) in items.iter().enumerate() {
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

        Ok(())
    }

    pub async fn read(args: Cli) -> Result<AppConfig> {
        let mut cfg = parse::parse(&args)?;

        // set socket path explicitly here
        // NOTE: this workaround exists due to a limitation in serde
        // see: https://github.com/serde-rs/serde/issues/2249
        cfg.socket = Some(match args.socket {
            Some(socket_path) => socket_path,
            None => get_socket_path(cfg.socket.as_ref())?,
        });

        // config validation
        {
            // sort items as defined in the configuration
            Self::sort(&mut cfg.items);

            // check no duplicate names
            Self::validate_names(&cfg.items)?;

            // check no empty powerline config
            cfg.theme.validate()?;
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::item::Common;
    use crate::i3::I3Item;

    macro_rules! item {
        () => {
            Item::new(Default::default(), I3Item::empty())
        };
        ($name:expr) => {
            Item::new(
                Common {
                    name: Some($name.into()),
                    ..Default::default()
                },
                I3Item::empty(),
            )
        };
        ($name:expr, $index:expr) => {
            Item::new(
                Common {
                    index: Some($index),
                    name: Some($name.into()),
                    ..Default::default()
                },
                I3Item::empty(),
            )
        };
    }

    #[test]
    fn validate_names() {
        AppConfig::validate_names(&[item!(), item!()]).unwrap();
        AppConfig::validate_names(&[item!("a"), item!("b")]).unwrap();
        AppConfig::validate_names(&[item!(), item!("a"), item!("b"), item!("c"), item!()]).unwrap();
    }

    #[test]
    #[should_panic(
        expected = "item names must be unique, item[3] and item[1] share the same name: c"
    )]
    fn validate_names_duplicate() {
        AppConfig::validate_names(&[item!("a"), item!("c"), item!("d"), item!("c")]).unwrap();
    }

    macro_rules! to_names {
        ($items:expr) => {
            $items
                .into_iter()
                .map(|i| i.name().to_owned())
                .collect::<Vec<_>>()
        };
    }

    #[test]
    fn sort_does_nothing() {
        let mut items = [item!("a"), item!("b"), item!("c")];
        AppConfig::sort(&mut items);
        assert_eq!(to_names!(items), ["a", "b", "c"]);
    }

    #[test]
    fn sort_one_index() {
        let mut items = [item!("a"), item!("b", 0), item!("c")];
        AppConfig::sort(&mut items);
        assert_eq!(to_names!(items), ["b", "a", "c"]);
    }

    #[test]
    fn sort_all_same_index() {
        let mut items = [item!("a", 0), item!("b", 0), item!("c", 0)];
        AppConfig::sort(&mut items);
        assert_eq!(to_names!(items), ["c", "b", "a"]);
    }

    #[test]
    fn sort_oob_index() {
        let mut items = [item!("a", 42), item!("b", 1729), item!("c", 9001)];
        AppConfig::sort(&mut items);
        assert_eq!(to_names!(items), ["b", "a", "c"]);
    }

    #[test]
    fn sort_reverse() {
        let mut items = [item!("a", 2), item!("b", 1), item!("c", 0)];
        AppConfig::sort(&mut items);
        assert_eq!(to_names!(items), ["a", "b", "c"]);
    }
}
