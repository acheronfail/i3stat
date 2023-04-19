use chrono::prelude::*;

use super::{
    Item,
    ToItem,
};

pub struct Time {
    full_format: String,
    short_format: String,
}

impl Default for Time {
    fn default() -> Self {
        Time {
            full_format: "%Y-%m-%d %H:%M:%S".into(),
            short_format: "%m/%d %H:%M".into(),
        }
    }
}

impl ToItem for Time {
    fn to_item(&self) -> Item {
        let now = Local::now();
        Item::new(now.format(&self.full_format).to_string())
            .short_text(now.format(&self.short_format).to_string())
    }
}
