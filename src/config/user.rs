use std::error::Error;
use std::path::PathBuf;

use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;
use serde_derive::{Deserialize, Serialize};

use super::item::Item;
use crate::cli::Cli;
use crate::ipc::get_socket_path;
use crate::theme::Theme;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Path to the socket to use for ipc. Useful when having multiple bars to separate their sockets.
    /// The CLI option takes precedence over this.
    socket: Option<PathBuf>,
    /// Specify the colours of the theme
    #[serde(default)]
    pub theme: Theme,
    /// List of the items for the bar - ordered left to right.
    pub items: Vec<Item>,
}

impl UserConfig {
    pub fn socket(&self) -> PathBuf {
        // SAFETY: when creating instances of `AppConfig` this option is always filled
        self.socket.clone().unwrap()
    }

    pub async fn read(args: Cli) -> Result<UserConfig, Box<dyn Error>> {
        let path = args
            .config
            .map(|p| p.with_extension(""))
            .or_else(|| dirs::config_dir().map(|d| d.join("istat/config")))
            .ok_or_else(|| "failed to find config dir")?;

        let mut cfg = Figment::new()
            .merge(Toml::file(path.with_extension("toml")))
            .merge(Yaml::file(path.with_extension("yaml")))
            .merge(Json::file(path.with_extension("json")))
            .extract::<UserConfig>()?;

        // set socket path
        cfg.socket = Some(match args.socket {
            Some(socket_path) => socket_path,
            None => get_socket_path(cfg.socket.as_ref())?,
        });

        // config validation
        {
            // check no duplicate names
            for (i, a) in cfg.items.iter().enumerate().rev() {
                for (j, b) in cfg.items.iter().enumerate() {
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
            if cfg.theme.powerline.len() <= 1 {
                return Err("theme.powerline must contain at least two values".into());
            }
        }

        Ok(cfg)
    }
}
