use serde_derive::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

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

#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum I3Button {
    Left = 1,
    Middle = 2,
    Right = 3,
    // TODO: verify these are in the right order
    ScrollDown = 4,
    ScrollUp = 5,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub name: Option<String>,
    pub instance: Option<String>,
    pub button: I3Button,
    pub modifiers: Vec<I3Modifier>,
    pub x: usize,
    pub y: usize,
    pub relative_x: usize,
    pub relative_y: usize,
    pub output_x: usize,
    pub output_y: usize,
    pub width: usize,
    pub height: usize,
}
