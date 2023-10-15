use std::convert::Infallible;

use tokio::io::{stdin, AsyncBufReadExt, BufReader};

use super::{I3ClickEvent, I3Item};
use crate::bar::Bar;
use crate::config::item::{Action, ActionWrapper, Actions};
use crate::config::AppConfig;
use crate::context::BarEvent;
use crate::dispatcher::Dispatcher;
use crate::error::Result;
use crate::i3::I3Button;
use crate::util::exec::exec;
use crate::util::RcCell;

pub async fn handle_click_events(
    bar: RcCell<Bar>,
    config: RcCell<AppConfig>,
    dispatcher: RcCell<Dispatcher>,
) -> Result<Infallible> {
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
        log::trace!("i3 click: {}", &line);
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

        // handle any custom actions
        if let Some(Actions {
            left_click,
            middle_click,
            right_click,
        }) = &config.items[idx].common.actions
        {
            let did_action = match click.button {
                I3Button::Left => handle_actions(left_click.as_ref(), &click, &bar[idx]),
                I3Button::Middle => handle_actions(middle_click.as_ref(), &click, &bar[idx]),
                I3Button::Right => handle_actions(right_click.as_ref(), &click, &bar[idx]),
                _ => false,
            };

            if did_action {
                log::debug!(
                    "not forwarding click event to item {} because custom action was run",
                    idx
                );
                continue;
            }
        }

        // send click event to the bar item
        if let Err(e) = dispatcher.send_bar_event(idx, BarEvent::Click(click)).await {
            log::warn!("{}", e);
            continue;
        }
    }
}

fn handle_actions(actions: Option<&ActionWrapper>, click: &I3ClickEvent, item: &I3Item) -> bool {
    let mut did_action = false;
    let actions = match actions {
        Some(ActionWrapper::Single(action)) => vec![action.clone()],
        Some(ActionWrapper::Many(actions)) => actions.clone(),
        None => return did_action,
    };

    for action in actions {
        let command = match action {
            Action::Simple(command) => Some(command),
            Action::WithOptions { command, modifiers } if modifiers == click.modifiers => {
                Some(command)
            }
            _ => None,
        };

        if let Some(command) = command {
            exec(command, item);
            did_action = true;
        }
    }

    did_action
}
