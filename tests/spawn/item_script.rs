use istat::i3::I3Button;
use serde_json::json;

use crate::util::TestProgram;

spawn_test!(
    script_simple,
    json!({
        "items":[
            {
                "type": "script",
                "command": "echo -n `if [ ! -z $I3_BUTTON ]; then echo button=$I3_BUTTON; else echo bar item; fi`",
                "output": "simple",
            }
        ]
    }),
    |mut istat: TestProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                {
                    "instance": "0",
                    "name": "script",
                    "full_text": "bar item",
                }
            ])
        );

        istat.click("0", I3Button::Left, &[]);
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                {
                    "instance": "0",
                    "name": "script",
                    "full_text": "button=1",
                }
            ])
        );

        istat.click("0", I3Button::Middle, &[]);
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                {
                    "instance": "0",
                    "name": "script",
                    "full_text": "button=2",
                }
            ])
        );
    }
);

spawn_test!(
    script_json,
    json!({
        "items":[
            {
                "type": "script",
                "command": r#"echo -n '{"full_text":"G'"'"'day"}'"#,
                "output": "json",
                "markup": "pango"
            }
        ]
    }),
    |mut istat: TestProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                {
                    "instance": "0",
                    "name": "script",
                    "full_text": "G'day",
                    "markup": "pango"
                }
            ])
        );
    }
);
