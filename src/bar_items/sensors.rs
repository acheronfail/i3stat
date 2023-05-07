use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use sysinfo::{ComponentExt, SystemExt};
use tokio::time::sleep;

use crate::context::{BarItem, Context};
use crate::i3::I3Item;

// TODO: store list of references to Components, so don't have to iter?
pub struct Sensors {
    interval: Duration,
}

impl Default for Sensors {
    fn default() -> Self {
        Sensors {
            interval: Duration::from_secs(2),
        }
    }
}

#[async_trait(?Send)]
impl BarItem for Sensors {
    async fn start(self: Box<Self>, ctx: Context) -> Result<(), Box<dyn Error>> {
        loop {
            let temp = {
                let mut state = ctx.state.lock().unwrap();
                // TODO: support choosing particular one
                state
                    .sys
                    .components_mut()
                    .iter_mut()
                    .find_map(|c| {
                        if c.label() == "coretemp Package id 0" {
                            c.refresh();
                            Some(c.temperature())
                        } else {
                            None
                        }
                    })
                    .unwrap()
            };

            ctx.update_item(I3Item::new(format!("TMP: {:.0}°C", temp)))
                .await?;

            sleep(self.interval).await;
        }
    }
}
