use serde_json::json;

use crate::i3::X11Test;

macro_rules! screenshot {
    ($item_json:expr) => {
        x_test!(
            hello_world,
            json!({ "items": [$item_json] }),
            |x_test: X11Test| {
                x_test.screenshot("bar-0");
            }
        );
    };
}

// TODO: would be nice to have each test generate screenshots, but we can't test everything yet
screenshot!(json!({
    "type": "raw",
    "full_text": "Hello, World!",
    "color": "#ff0000"
}));
