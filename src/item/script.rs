use std::collections::HashMap;

use async_trait::async_trait;
use tokio::process::Command;

use super::{BarItem, Item};
use crate::context::Context;

pub struct Script {
    command: String,
}

impl Default for Script {
    fn default() -> Self {
        Script {
            command: "echo -n `if [ ! -z $I3_BUTTON ]; then echo button=$I3_BUTTON; else echo bar item; fi`".into(),
        }
    }
}

impl Script {
    // returns stdout
    async fn run(&self, env: &HashMap<&str, String>) -> String {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .envs(env)
            .output()
            .await
            .unwrap();

        String::from_utf8_lossy(&output.stdout).to_string()
    }
}

#[async_trait]
impl BarItem for Script {
    async fn start(&mut self, mut ctx: Context) {
        // TODO: set interval and run multiple times based on interval
        // https://docs.rs/tokio/latest/tokio/time/fn.interval.html
        // TODO: potentially have scripts that are never run again? no click events, etc

        let mut env = HashMap::new();

        loop {
            // Initial run has no click environment variables
            let stdout = self.run(&env).await;
            ctx.update_item(Item::new(stdout)).await.unwrap();

            // On any click event, update the environment map and re-run the script
            if let Some(click) = ctx.wait_for_click().await {
                click.name.inspect(|name| {
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
