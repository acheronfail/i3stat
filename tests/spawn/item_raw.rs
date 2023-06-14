use serde_json::json;

use crate::util::TestProgram;

spawn_test!(
    raw,
    json!({
        "items": [
            { "type": "raw", "full_text": "0" },
            { "type": "raw", "full_text": "1" },
            { "type": "raw", "full_text": "2", "name": "custom_name" },
        ]
    }),
    |mut istat: TestProgram| {
        istat.wait_for_all_init();

        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "full_text": "0", "name": "raw" },
                { "instance": "1", "full_text": "1", "name": "raw" },
                { "instance": "2", "full_text": "2", "name": "custom_name" },
            ])
        );
    }
);
