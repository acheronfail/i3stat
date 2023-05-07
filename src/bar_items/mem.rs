use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use sysinfo::SystemExt;
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::i3::I3Item;

pub struct Mem {
    interval: Duration,
}

impl Default for Mem {
    fn default() -> Self {
        Mem {
            interval: Duration::from_secs(5),
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Mem {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let available = {
                let mut state = ctx.state.lock().unwrap();
                state.sys.refresh_memory();
                state.sys.available_memory()
            };

            ctx.update_item(I3Item::new(format!(
                "MEM: {}",
                ByteSize(available).to_string_as(false)
            )))
            .await?;

            sleep(self.interval).await;
        }
    }
}
