use serde_json::json;

use crate::util::{Test, TestProgram};

spawn_test!(
    krb_on,
    json!({
        "items": [
            {
                "type": "krb",
            }
        ]
    }),
    |test: &mut Test| {
        test.add_bin("klist", "#!/usr/bin/env bash\nexit 0");
    },
    |mut istat: TestProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "krb", "full_text": "󱕵", "markup": "pango", "color": "#D8DEE9" }])
        );
    }
);

spawn_test!(
    krb_off,
    json!({
        "items": [
            {
                "type": "krb",
            }
        ]
    }),
    |test: &mut Test| {
        test.add_bin("klist", "#!/usr/bin/env bash\nexit 1");
    },
    |mut istat: TestProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "krb", "full_text": "󱕵", "markup": "pango", "color": "#4C566A" }])
        );
    }
);
