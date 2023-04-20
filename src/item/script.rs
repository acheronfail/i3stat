use async_trait::async_trait;
use tokio::process::Command;

use super::{
    BarItem,
    Item,
    Sender,
};
use crate::context::Ctx;

pub struct Script {
    command: String,
}

impl Default for Script {
    fn default() -> Self {
        Script {
            command: "echo -n Hello, World!".into(),
        }
    }
}

#[async_trait]
impl BarItem for Script {
    async fn start(&self, _: Ctx, tx: Sender) {
        // TODO: set interval and run multiple times based on interval
        // https://docs.rs/tokio/latest/tokio/time/fn.interval.html

        let output = Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .output()
            .await
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        tx.send(Item::new(stdout)).await.unwrap();
    }
}
