use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use tokio::process::Command;

use crate::context::{BarItem, Context};
use crate::i3::I3Item;

#[allow(dead_code)]
pub enum ScriptFormat {
    Simple,
    Json,
}

pub struct Script {
    command: String,
    format: ScriptFormat,
}

impl Default for Script {
    fn default() -> Self {
        Script {
            command: "echo -n `if [ ! -z $I3_BUTTON ]; then echo button=$I3_BUTTON; else echo bar item; fi`".into(),
            format: ScriptFormat::Simple
        }
    }
}

impl Script {
    // returns stdout
    async fn run(&self, env: &HashMap<&str, String>) -> Result<String, Box<dyn Error>> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .envs(env)
            .output()
            .await?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[async_trait(?Send)]
impl BarItem for Script {
    async fn start(self: Box<Self>, mut ctx: Context) -> Result<(), Box<dyn Error>> {
        // TODO: set interval and run multiple times based on interval
        // https://docs.rs/tokio/latest/tokio/time/fn.interval.html
        // TODO: potentially have scripts that are never run again? no click events, etc
        // TODO: what happens if script execution is longer than the configured interval?

        let name = format!(
            "script({}...)",
            self.command.chars().take(10).collect::<String>()
        );

        let mut env = HashMap::new();

        loop {
            // Initial run has no click environment variables
            let stdout = self.run(&env).await?;
            let mut item = match self.format {
                ScriptFormat::Simple => I3Item::new(stdout),
                ScriptFormat::Json => match serde_json::from_str(&stdout) {
                    Ok(item) => item,
                    Err(e) => {
                        dbg!(e); // TODO: error logging
                        I3Item::new("ERR").background_color(ctx.theme.error)
                    }
                },
            };
            item = item.name(&name);
            ctx.update_item(item).await?;

            // On any click event, update the environment map and re-run the script
            if let Some(click) = ctx.wait_for_click().await {
                click.name.map(|name| {
                    env.insert("I3_NAME", name.to_string());
                });
                env.insert(
                    "I3_MODIFIERS",
                    click
                        .modifiers
                        .iter()
                        .map(|m| serde_json::to_string(m).unwrap())
                        .collect::<Vec<_>>()
                        .join(","),
                );
                env.insert("I3_BUTTON", serde_json::to_string(&click.button).unwrap());
                env.insert("I3_X", click.x.to_string());
                env.insert("I3_Y", click.y.to_string());
                env.insert("I3_RELATIVE_X", click.relative_x.to_string());
                env.insert("I3_RELATIVE_Y", click.relative_y.to_string());
                env.insert("I3_OUTPUT_X", click.output_x.to_string());
                env.insert("I3_OUTPUT_Y", click.output_y.to_string());
                env.insert("I3_WIDTH", click.width.to_string());
                env.insert("I3_HEIGHT", click.height.to_string());
            }
        }
    }
}
