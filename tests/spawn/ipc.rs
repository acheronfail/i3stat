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
        assert_eq!(istat.next_line().unwrap(), None);
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
    refresh_all,
    json!({
        "items": [
            { "type": "raw", "full_text": "0" },
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" }
        ]
    }),
    |mut istat: TestProgram| {
        istat.wait_for_all_init();

        // initial state
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "raw", "full_text": "0" },
                { "instance": "1", "name": "script", "full_text": "signal: false" },
            ])
        );

        // send refresh
        assert_eq!(
            istat.send_ipc(IpcMessage::RefreshAll),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        // we only expect a single update - "raw" items don't update
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "raw", "full_text": "0" },
                { "instance": "1", "name": "script", "full_text": "signal: true" },
            ])
        );

        istat.send_shutdown();
        assert_eq!(istat.next_line_json().unwrap(), json!(null));
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
