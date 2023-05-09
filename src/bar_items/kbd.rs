use std::error::Error;

use async_trait::async_trait;
use strum::{EnumIter, IntoEnumIterator};
use tokio::fs;

use crate::context::{BarItem, Context};
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;
use crate::BarEvent;

#[derive(Debug, Default)]
pub struct Kbd {}

#[derive(Debug, EnumIter, PartialEq, Eq)]
enum Keys {
    CapsLock,
    NumLock,
}

impl Keys {
    fn sys_dir_suffix(&self) -> &'static str {
        match self {
            Keys::CapsLock => "::capslock",
            Keys::NumLock => "::numlock",
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Keys::CapsLock => "C",
            Keys::NumLock => "N",
        }
    }

    async fn is_on(&self) -> Result<bool, Box<dyn Error>> {
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
            None => todo!("handle when not found"),
        }
    }

    async fn format(self, theme: &Theme) -> Result<String, Box<dyn Error>> {
        let is_on = self.is_on().await?;
        Ok(format!(
            r#"<span foreground="{}">{}</span>"#,
            if is_on { theme.success } else { theme.error },
            self.symbol()
        ))
    }
}

#[async_trait(?Send)]
impl BarItem for Kbd {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        'outer: loop {
            let text = futures::future::join_all(Keys::iter().map(|k| k.format(&ctx.theme)))
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .join("");

            let item = I3Item::new(text).name("kbd").markup(I3Markup::Pango);
            ctx.update_item(item).await?;

            // wait for a signal and then refresh
            loop {
                if let Some(BarEvent::Signal) = ctx.wait_for_event().await {
                    continue 'outer;
                }
            }
        }
    }
}
