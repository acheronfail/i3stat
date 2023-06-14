mod util;

use serde_json::json;

use self::util::TestProgram;

spawn_test!(
    shuts_down_on_ipc,
    json!({ "items":[] }),
    |mut istat: TestProgram| {
        istat.assert_i3_header();
        istat.shutdown();
        istat.assert_next_line(None);
    }
);

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
    |mut istat: TestProgram| {
        istat.assert_i3_header();
        istat.assert_next_line_json(json!([
            {
                "instance": "0",
                "name": "time",
                "full_text": "󰥔 1985-10-26 01:35:00",
                "short_text": "01:35",
                "markup": "pango"
            }
        ]));
        istat.assert_next_line_json(json!([
            {
                "instance": "0",
                "name": "time",
                "full_text": "󰥔 1985-10-26 01:35:01",
                "short_text": "01:35",
                "markup": "pango"
            }
        ]));
        istat.shutdown();
        istat.assert_next_line_json(json!(null));
    }
);
