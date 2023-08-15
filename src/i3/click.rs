use serde_derive::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Default, Copy, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum I3Button {
    #[default]
    Left = 1,
    Middle = 2,
    Right = 3,
    ScrollUp = 4,
    ScrollDown = 5,
    ScrollRight = 6,
    ScrollLeft = 7,
    // apparently the maximum number of mouse buttons is 24!
    // see: https://www.x.org/releases/current/doc/man/man4/mousedrv.4.xhtml
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
