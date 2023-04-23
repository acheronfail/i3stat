use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use futures::future;
use tokio::fs::read_to_string;
use tokio::time::sleep;

use super::{BarItem, Item};
use crate::context::Context;

struct Bat(PathBuf);

impl Bat {
    fn name(&self) -> String {
        self.0.file_name().unwrap().to_string_lossy().into_owned()
    }

    async fn get_charge(&self) -> Result<f32, Box<dyn Error>> {
        macro_rules! get_usize {
            ($x: expr) => {
                read_to_string(self.0.join($x))
                    .await?
                    .trim()
                    .parse::<usize>()? as f32
            };
        }

        Ok(get_usize!("charge_now") / get_usize!("charge_full") * 100.0)
    }
}

pub struct Battery {
    interval: Duration,
    batteries: Vec<Bat>,
}

impl Default for Battery {
    fn default() -> Self {
        let battery_dir = PathBuf::from("/sys/class/power_supply");
        let batteries = std::fs::read_dir(&battery_dir)
            .unwrap()
            .into_iter()
            .filter_map(|res| {
                res.ok()
                    .and_then(|ent| match ent.file_type() {
                        Ok(ft) if ft.is_symlink() => Some(battery_dir.join(ent.file_name())),
                        _ => None,
                    })
                    .and_then(|dir| {
                        if dir.join("charge_now").exists() {
                            Some(Bat(dir))
                        } else {
                            None
                        }
                    })
            })
            .collect::<Vec<_>>();

        Battery {
            interval: Duration::from_secs(5),
            batteries,
        }
    }
}

impl Battery {
    async fn map(bat: &Bat) -> String {
        format!("{}:{:.0}%", bat.name(), bat.get_charge().await.unwrap())
    }
}

#[async_trait]
impl BarItem for Battery {
    async fn start(&mut self, ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            ctx.update_item(Item::new(
                future::join_all(self.batteries.iter().map(Battery::map))
                    .await
                    .join(", "),
            ))
            .await?;

            sleep(self.interval).await;
        }
    }
}
