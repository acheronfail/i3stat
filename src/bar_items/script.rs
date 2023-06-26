use std::collections::HashMap;
use crate::error::Result;
use std::time::Duration;

use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use tokio::process::Command;

use crate::context::{BarEvent, BarItem, Context, StopAction};
use crate::i3::{I3Item, I3Markup};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ScriptFormat {
    #[default]
    Simple,
    Json,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Script {
    pub command: String,
    #[serde(default)]
    pub output: ScriptFormat,
    #[serde(default, with = "humantime_serde")]
    interval: Option<Duration>,
    #[serde(default)]
    pub markup: I3Markup,
}

impl Script {
    // returns stdout
    async fn run(&self, env: &HashMap<&str, String>) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .envs(env)
            .output()
            .await?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait(?Send)]
impl BarItem for Script {
    async fn start(&self, mut ctx: Context) -> Result<StopAction> {
        // update script environment on any click event
        let mut script_env = HashMap::new();
        let handle_event = |event: BarEvent, env: &mut HashMap<_, _>| match event {
            BarEvent::Signal => {
                env.insert("I3_SIGNAL", "true".to_string());
            }
            BarEvent::Click(c) => {
                env.remove("I3_SIGNAL");
                c.name.map(|name| {
                    env.insert("I3_NAME", name.to_string());
                });
                env.insert(
                    "I3_MODIFIERS",
                    c.modifiers
                        .iter()
                        // SAFETY: if these types don't serialise then things would have gone wrong previously
                        .map(|m| serde_json::to_string(m).unwrap())
                        .collect::<Vec<_>>()
                        .join(","),
                );
                // SAFETY: if these types don't serialise then things would have gone wrong previously
                env.insert("I3_BUTTON", serde_json::to_string(&c.button).unwrap());
                env.insert("I3_X", c.x.to_string());
                env.insert("I3_Y", c.y.to_string());
                env.insert("I3_RELATIVE_X", c.relative_x.to_string());
                env.insert("I3_RELATIVE_Y", c.relative_y.to_string());
                env.insert("I3_OUTPUT_X", c.output_x.to_string());
                env.insert("I3_OUTPUT_Y", c.output_y.to_string());
                env.insert("I3_WIDTH", c.width.to_string());
                env.insert("I3_HEIGHT", c.height.to_string());
            }
            _ => {}
        };

        loop {
            // Initial run has no click environment variables
            let stdout = self.run(&script_env).await?;
            let mut item = match self.output {
                ScriptFormat::Simple => I3Item::new(stdout),
                ScriptFormat::Json => match serde_json::from_str(&stdout) {
                    Ok(item) => item,
                    Err(e) => {
                        log::error!("failed to parse script json output: {}", e);
                        I3Item::new("ERROR").background_color(ctx.config.theme.red)
                    }
                },
            };
            item = item.markup(self.markup);

            ctx.update_item(item).await?;

            match self.interval {
                // if an interval is set, then re-run the script on that interval
                Some(interval) => {
                    ctx.delay_with_event_handler(interval, |event| {
                        handle_event(event, &mut script_env);
                        async {}
                    })
                    .await
                }
                // if not, re-run the script on any event
                None => {
                    if let Some(event) = ctx.wait_for_event(None).await {
                        handle_event(event, &mut script_env);
                    }
                }
            }
        }
    }
}
