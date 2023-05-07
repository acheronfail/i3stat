use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use bytesize::ByteSize;
use sysinfo::{NetworkExt, NetworksExt, SystemExt};
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::i3::I3Item;

pub struct NetUsage {
    interval: Duration,
}

impl Default for NetUsage {
    fn default() -> Self {
        NetUsage {
            interval: Duration::from_secs(1),
        }
    }
}

#[async_trait(?Send)]
impl BarItem for NetUsage {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let (up, down) = {
                let mut state = ctx.state.lock().unwrap();
                state.sys.refresh_networks();
                state
                    .sys
                    .networks()
                    .iter()
                    .fold((0, 0), |(d, u), (_, net)| {
                        (d + net.received(), u + net.transmitted())
                    })
            };

            ctx.update_item(I3Item::new(format!(
                "↓{} ↑{}",
                ByteSize(down).to_string_as(true),
                ByteSize(up).to_string_as(true)
            )))
            .await?;

            sleep(self.interval).await;
        }
    }
}
