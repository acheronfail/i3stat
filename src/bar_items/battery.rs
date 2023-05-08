use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use futures::future;
use tokio::fs::read_to_string;
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

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
    fn format(theme: &Theme, name: &String, pct: f32) -> (String, String) {
        let (icon, fg) = match pct as u32 {
            0..=15 => ("", Some(theme.error)),
            16..=25 => ("", Some(theme.danger)),
            26..=50 => ("", Some(theme.warning)),
            51..=75 => ("", None),
            76..=u32::MAX => ("", Some(theme.success)),
        };

        let name = if name == "BAT0" { icon } else { name.as_str() };
        let fg = fg.map(|c| c.to_string()).unwrap_or("".into());
        (
            format!(r#"<span foreground="{}">{} {:.0}%</span>"#, fg, name, pct),
            format!(r#"<span foreground="{}">{:.0}%</span>"#, fg, pct),
        )
    }

    async fn get(bat: &Bat) -> Result<(String, f32), Box<dyn Error>> {
        Ok((bat.name(), bat.get_charge().await?))
    }
}

#[async_trait(?Send)]
impl BarItem for Battery {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let batteries = future::join_all(self.batteries.iter().map(Battery::get))
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;

            let len = batteries.len();
            let (full, short) = batteries.into_iter().fold(
                (Vec::with_capacity(len), Vec::with_capacity(len)),
                |mut acc, (name, pct)| {
                    let (full, short) = Self::format(&ctx.theme, &name, pct);
                    acc.0.push(full);
                    acc.1.push(short);
                    acc
                },
            );

            let item = I3Item::new(full.join(", "))
                .short_text(short.join(", "))
                .name("bat")
                .markup(I3Markup::Pango);

            ctx.update_item(item).await?;
            sleep(self.interval).await;
        }
    }
}
