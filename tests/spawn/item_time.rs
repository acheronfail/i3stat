use serde_json::json;

use crate::spawn::SpawnedProgram;

const TIME_LONG: &str = "%Y-%m-%d %H:%M:%S";
const TIME_SHORT: &str = "%H:%M";

spawn_test!(
    time,
    json!({
        "items":[
            {
                "type": "time",
                "interval": "1 s",
                "format_long": TIME_LONG,
                "format_short": TIME_SHORT
            }
        ]
    }),
    |mut i3stat: SpawnedProgram| {
        assert_eq!(
            i3stat.next_line_json().unwrap(),
            json!([
                {
                    "instance": "0",
                    "name": "time",
                    "full_text": "󰥔 1985-10-26 01:35:00",
                    "short_text": "01:35",
                    "markup": "pango"
                }
            ])
        );
        assert_eq!(
            i3stat.next_line_json().unwrap(),
            json!([
                {
                    "instance": "0",
                    "name": "time",
                    "full_text": "󰥔 1985-10-26 01:35:01",
                    "short_text": "01:35",
                    "markup": "pango"
                }
            ])
        );
        i3stat.send_shutdown();
        assert_eq!(i3stat.next_line_json().unwrap(), json!(null));
    }
);
