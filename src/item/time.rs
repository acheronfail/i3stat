use chrono::prelude::*;

use super::{Item, ToItem};

pub struct Time {
    format: String,
}

impl Default for Time {
    fn default() -> Self {
        Time {
            format: "%Y-%m-%d %H:%M:%S".into(),
        }
    }
}

impl ToItem for Time {
    fn to_item(&self) -> Item {
        Item::text(Local::now().format(&self.format).to_string())
    }
}
