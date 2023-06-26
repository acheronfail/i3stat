use std::time::Duration;

use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use tokio::fs;

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::error::Result;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Kbd {
    show: Option<Vec<Keys>>,
    #[serde(default, with = "crate::human_time::option")]
    interval: Option<Duration>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Keys {
    CapsLock,
    NumLock,
    ScrollLock,
}

impl Keys {
    fn sys_dir_suffix(&self) -> &'static str {
        match self {
            Keys::CapsLock => "::capslock",
            Keys::NumLock => "::numlock",
            Keys::ScrollLock => "::scrolllock",
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Keys::CapsLock => "C",
            Keys::NumLock => "N",
            Keys::ScrollLock => "S",
        }
    }

    async fn is_on(&self) -> Result<bool> {
        let mut entries = fs::read_dir("/sys/class/leds/").await?;
        let suffix = self.sys_dir_suffix();

        let mut dir = None;
        while let Some(entry) = entries.next_entry().await? {
            let ty = entry.file_type().await?;
            if !(ty.is_dir() || ty.is_symlink()) {
                continue;
            }

            if entry.file_name().to_string_lossy().ends_with(suffix) {
                dir = Some(entry.path());
                break;
            }
        }

        match dir {
            Some(path) => {
                let brightness = path.join("brightness");
                let value: u32 = fs::read_to_string(&brightness).await?.trim().parse()?;
                Ok(value == 1)
            }
            None => {
                let name = serde_json::to_string(&self)?;
                Err(format!("failed to find led file for: {}", name).into())
            }
        }
    }

    async fn format(self, theme: &Theme) -> Result<String> {
        Ok(match self.is_on().await {
            Ok(is_on) => format!(
                r#"<span foreground="{}">{}</span>"#,
                if is_on { theme.fg } else { theme.dim },
                self.symbol()
            ),
            Err(e) => {
                log::error!("{}", e);
                format!(
                    r#"<span background="{}" foreground="{}">{}</span>"#,
                    theme.red,
                    theme.bg,
                    self.symbol()
                )
            }
        })
    }
}

#[async_trait(?Send)]
impl BarItem for Kbd {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        let keys = self.show.clone().unwrap_or_else(|| Keys::iter().collect());

        'outer: loop {
            let text = futures::future::join_all(keys.iter().map(|k| k.format(&ctx.config.theme)))
                .await
                .into_iter()
                .collect::<Result<Vec<_>>>()?
                .join("");

            let item = I3Item::new(text).markup(I3Markup::Pango);
            ctx.update_item(item).await?;

            // wait for a signal and then refresh
            loop {
                if let Some(BarEvent::Signal) = ctx.wait_for_event(self.interval).await {
                    continue 'outer;
                }
            }
        }
    }
}
