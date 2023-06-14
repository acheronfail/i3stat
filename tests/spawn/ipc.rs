use istat::ipc::IpcMessage;
use serde_json::json;

use crate::util::TestProgram;

spawn_test!(
    shutdown,
    json!({ "items": [] }),
    |mut istat: TestProgram| {
        // request shutdown
        istat.send_shutdown();
        // there were no items in the config, so nothing should have been outputted
        assert_eq!(istat.next_line(), None);
    }
);

spawn_test!(
    info,
    json!({
        "items": [
            { "type": "raw", "full_text": "0" },
            { "type": "raw", "full_text": "1" },
            { "type": "raw", "full_text": "2", "name": "custom_name" },
        ]
    }),
    |mut istat: TestProgram| {
        assert_eq!(
            istat.send_ipc(IpcMessage::Info),
            json!({
                "info": {
                    "0": "raw",
                    "1": "raw",
                    "2": "custom_name",
                }
            })
        );
    }
);

spawn_test!(
    refresh_all_raw_skipped,
    json!({
        "items": [
            // NOTE: raw items aren't updated - they don't listen to anything
            { "type": "raw", "full_text": "0" },
            { "type": "raw", "full_text": "1" }
        ]
    }),
    |mut istat: TestProgram| {
        // NOTE: skip first line - an update is printed per item update, and multiple raw items mean multiple updates
        // see the test in the `item_raw` mod
        istat.next_line_json();

        assert_eq!(
            istat.next_line_json(),
            json!([
                { "instance": "0", "full_text": "0", "name": "raw" },
                { "instance": "1", "full_text": "1", "name": "raw" },
            ])
        );

        assert_eq!(
            istat.send_ipc(IpcMessage::RefreshAll),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        istat.send_shutdown();

        // raw items don't update, so nothing else should be outputted
        assert_eq!(istat.next_line_json(), json!(null));
    }
);

// spawn_test!(
//     signal_item,
//     json!({ "items": [] }),
//     |mut istat: TestProgram| { todo!() }
// );

// spawn_test!(
//     send_click,
//     json!({ "items": [] }),
//     |mut istat: TestProgram| { todo!() }
// );

// spawn_test!(
//     get_config,
//     json!({ "items": [] }),
//     |mut istat: TestProgram| { todo!() }
// );

// spawn_test!(
//     get_theme,
//     json!({ "items": [] }),
//     |mut istat: TestProgram| { todo!() }
// );

// spawn_test!(
//     set_theme,
//     json!({ "items": [] }),
//     |mut istat: TestProgram| { todo!() }
// );
