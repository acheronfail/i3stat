use std::convert::Infallible;
use std::error::Error;

use tokio::io::{stdin, AsyncBufReadExt, BufReader};

use super::I3ClickEvent;
use crate::context::BarEvent;
use crate::dispatcher::Dispatcher;
use crate::util::RcCell;

pub async fn handle_click_events(
    dispatcher: RcCell<Dispatcher>,
) -> Result<Infallible, Box<dyn Error>> {
    let s = BufReader::new(stdin());
    let mut lines = s.lines();
    loop {
        let mut line = lines
            .next_line()
            .await?
            .ok_or_else(|| "received unexpected end of STDIN")?;

        // skip opening array as part of the protocol
        if line.trim() == "[" {
            continue;
        }

        // skip over any preceding `,` as part of the protocol
        line = line
            .chars()
            .skip_while(|c| c.is_whitespace() || *c == ',')
            .collect();

        // parse click event (single line JSON)
        let click = serde_json::from_str::<I3ClickEvent>(&line)?;

        // parse bar item index from the "instance" property
        let idx = match click.instance.as_ref() {
            Some(inst) => match inst.parse::<usize>() {
                Ok(i) => i,
                Err(e) => {
                    log::warn!(
                        "failed to parse click 'instance' property: {}, error: {}",
                        inst,
                        e
                    );
                    continue;
                }
            },
            None => {
                log::warn!(
                    "received click event without 'instance' property, cannot route to item: {:?}",
                    click
                );
                continue;
            }
        };

        if let Err(e) = dispatcher.send_bar_event(idx, BarEvent::Click(click)).await {
            log::warn!("{}", e);
            continue;
        }
    }
}
