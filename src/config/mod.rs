mod item;
mod user;

use std::error::Error;
use std::path::PathBuf;

use indexmap::IndexMap;
use jq::JqProgram;
use tokio::sync::watch;

use self::item::Item;
use self::user::UserConfig;
use crate::cli::Cli;

pub struct RuntimeConfig {
    pub user: UserConfig,
    pub item_meta: Vec<ItemMeta>,
}

impl RuntimeConfig {
    pub async fn new(args: Cli) -> Result<RuntimeConfig, Box<dyn Error>> {
        let user = UserConfig::read(args).await?;
        let item_meta = user
            .items
            .iter()
            .map(|i| ItemMeta::new(i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RuntimeConfig { user, item_meta })
    }

    pub fn socket(&self) -> PathBuf {
        self.user.socket()
    }

    pub fn item_name_map(&self) -> IndexMap<usize, String> {
        let mut map = self
            .user
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| (idx, item.name().to_owned()))
            .collect::<IndexMap<usize, String>>();

        map.sort_keys();
        map
    }
}

pub struct ItemMeta {
    tx_pause: watch::Sender<bool>,
    rx_pause: watch::Receiver<bool>,
    enabled_when: Option<JqProgram>,
}

impl ItemMeta {
    fn new(item: &Item) -> Result<ItemMeta, Box<dyn Error>> {
        let (tx_pause, rx_pause) = watch::channel(false);
        Ok(ItemMeta {
            tx_pause,
            rx_pause,
            enabled_when: match item.common.enabled_when {
                Some(ref s) => Some(jq::compile(&s)?),
                None => None,
            },
        })
    }

    pub fn is_paused(&self) -> bool {
        *self.rx_pause.borrow()
    }

    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.rx_pause.clone()
    }

    pub fn should_remove(&mut self, payload: impl AsRef<str>) -> Result<bool, Box<dyn Error>> {
        if let Some(ref mut program) = self.enabled_when {
            let result = program.run(payload.as_ref())?;
            // TODO: a nice way to debug this somehow?
            // dbg!(&result);
            let pause = if result.trim() == "true" { true } else { false };
            if *self.tx_pause.borrow() != pause {
                self.tx_pause.send(pause)?;
            }

            Ok(pause)
        } else {
            Ok(false)
        }
    }
}
