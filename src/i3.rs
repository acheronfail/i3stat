use async_trait::async_trait;
use serde_derive::{
    Deserialize,
    Serialize,
};
use serde_repr::Deserialize_repr;

use crate::{
    context::Ctx,
    Sender,
};

#[derive(Debug, Serialize)]
pub struct I3BarHeader {
    version: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_signal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cont_signal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    click_events: Option<bool>,
}

impl Default for I3BarHeader {
    fn default() -> Self {
        I3BarHeader {
            version: 1,
            stop_signal: None,
            cont_signal: None,
            click_events: Some(true),
        }
    }
}

#[derive(Debug, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum I3Button {
    Left = 1,
    Middle = 2,
    Right = 3,
    // TODO: verify these are in the right order
    ScrollDown = 4,
    ScrollUp = 5,
}

#[derive(Debug, Deserialize)]
pub enum I3Modifier {
    Mod1,
    Mod2,
    Mod3,
    Mod4,
    Mod5,
    Shift,
    Control,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct I3ClickEvent {
    name: Option<String>,
    instance: Option<String>,
    button: I3Button,
    modifiers: Vec<I3Modifier>,
    x: usize,
    y: usize,
    relative_x: usize,
    relative_y: usize,
    output_x: usize,
    output_y: usize,
    width: usize,
    height: usize,
}

#[async_trait]
pub trait BarItem: Send {
    async fn start(&self, ctx: Ctx, tx: Sender);
}
