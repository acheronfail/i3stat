use serde_json::json;

use crate::i3::util::MouseButton::*;
use crate::i3::X11Test;

x_test!(
    click_on_item,
    json!({
        "items": [
            {
                "type": "script",
                "command": r##"echo -n '{"full_text":"btn='${I3_BUTTON:-none}'", "separator_block_width": 0, "separator": false, "background": "#800", "min_width": 200}'"##,
                "output": "json",
            },
            {
                "type": "script",
                "command": r##"echo -n '{"full_text":"btn='${I3_BUTTON:-none}'", "separator_block_width": 0, "separator": false, "background": "#088", "min_width": 200}'"##,
                "output": "json",
            }
        ]
    }),
    |x_test: &X11Test| {
        // get bar dimensions from i3
        let (x, y, w, _) = x_test.i3_get_bar_position("bar-0");

        // check initial state
        assert_json_contains!(
            x_test.istat_get_bar(),
            json!([
                { "instance": "0", "full_text": "btn=none" },
                { "instance": "1", "full_text": "btn=none" }
            ]),
        );

        // click on the items
        x_test.click(Left, x + w as i16, y);
        x_test.click(Right, x + (w - 200) as i16, y);

        // check item received the click
        assert_json_contains!(
            x_test.istat_get_bar(),
            json!([
                { "instance": "0", "full_text": "btn=3" },
                { "instance": "1", "full_text": "btn=1" }
            ])
        );
    }
);
