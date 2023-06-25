use istat::i3::{I3Button, I3ClickEvent};
use istat::ipc::protocol::{IpcBarEvent, IpcMessage};
use serde_json::{json, Value};

use crate::spawn::SpawnedProgram;

spawn_test!(
    shutdown,
    json!({ "items": [] }),
    |mut istat: SpawnedProgram| {
        // request shutdown
        istat.send_shutdown();
        // there were no items in the config, so nothing should have been outputted
        assert_eq!(istat.next_line().unwrap(), None);
        // check exit status
        let status = istat.child.wait().unwrap();
        assert_eq!(status.code(), Some(0));
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
    |mut istat: SpawnedProgram| {
        assert_eq!(
            istat.send_ipc(IpcMessage::Info),
            json!({
                "value": {
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
    |mut istat: SpawnedProgram| {
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

spawn_test!(
    signal_item_index,
    json!({
        "items": [
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
        ]
    }),
    |mut istat: SpawnedProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "signal: false" },
                { "instance": "1", "name": "script", "full_text": "signal: false" },
                { "instance": "2", "name": "script", "full_text": "signal: false" },
            ])
        );

        assert_eq!(
            istat.send_ipc(IpcMessage::BarEvent {
                instance: "1".into(),
                event: IpcBarEvent::Signal
            }),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "signal: false" },
                { "instance": "1", "name": "script", "full_text": "signal: true" },
                { "instance": "2", "name": "script", "full_text": "signal: false" },
            ])
        );
    }
);

spawn_test!(
    signal_item_name_first,
    json!({
        "items": [
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
        ]
    }),
    |mut istat: SpawnedProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "signal: false" },
                { "instance": "1", "name": "script", "full_text": "signal: false" },
                { "instance": "2", "name": "script", "full_text": "signal: false" },
            ])
        );

        assert_eq!(
            istat.send_ipc(IpcMessage::BarEvent {
                instance: "script".into(),
                event: IpcBarEvent::Signal
            }),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "signal: true" },
                { "instance": "1", "name": "script", "full_text": "signal: false" },
                { "instance": "2", "name": "script", "full_text": "signal: false" },
            ])
        );
    }
);

spawn_test!(
    signal_item_name_specific,
    json!({
        "items": [
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple" },
            { "type": "script", "command": "echo -n signal: ${I3_SIGNAL:-false}", "output": "simple", "name": "foo" },
        ]
    }),
    |mut istat: SpawnedProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "signal: false" },
                { "instance": "1", "name": "script", "full_text": "signal: false" },
                { "instance": "2", "name": "foo", "full_text": "signal: false" },
            ])
        );

        assert_eq!(
            istat.send_ipc(IpcMessage::BarEvent {
                instance: "foo".into(),
                event: IpcBarEvent::Signal
            }),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "signal: false" },
                { "instance": "1", "name": "script", "full_text": "signal: false" },
                { "instance": "2", "name": "foo", "full_text": "signal: true" },
            ])
        );
    }
);

spawn_test!(
    send_click,
    json!({
        "items": [
            {
                "type": "script",
                "command": "echo -n `if [ ! -z $I3_BUTTON ]; then echo button=$I3_BUTTON; else echo bar item; fi`",
                "output": "simple",
            }
        ]
    }),
    |mut istat: SpawnedProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "bar item" }
            ])
        );

        // ipc click with index
        assert_eq!(
            istat.send_ipc(IpcMessage::BarEvent {
                instance: "0".into(),
                event: IpcBarEvent::Click(I3ClickEvent {
                    button: I3Button::Left,
                    ..Default::default()
                })
            }),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "button=1" }
            ])
        );

        // ipc click with name
        assert_eq!(
            istat.send_ipc(IpcMessage::BarEvent {
                instance: "script".into(),
                event: IpcBarEvent::Click(I3ClickEvent {
                    button: I3Button::Right,
                    ..Default::default()
                })
            }),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([
                { "instance": "0", "name": "script", "full_text": "button=3" }
            ])
        );
    }
);

spawn_test!(
    get_config,
    json!({ "items": [{ "type": "raw", "full_text": "raw" }] }),
    |mut istat: SpawnedProgram| {
        let reply = istat.send_ipc(IpcMessage::GetConfig);
        let config = reply.get("value").unwrap();
        assert_eq!(config.get("items").unwrap().as_array().unwrap().len(), 1);
        assert!(config.get("socket").unwrap().is_string());
        assert!(config.get("theme").is_some());
    }
);

spawn_test!(
    get_theme,
    json!({ "items": [] }),
    |mut istat: SpawnedProgram| {
        let reply = istat.send_ipc(IpcMessage::GetConfig);
        let config = reply.get("value").unwrap();

        let reply = istat.send_ipc(IpcMessage::GetTheme);
        let theme = reply.get("value").unwrap();

        assert_eq!(config.get("theme").unwrap(), theme);
    }
);

spawn_test!(
    set_theme,
    json!({ "items": [] }),
    |mut istat: SpawnedProgram| {
        // get theme
        let mut reply = istat.send_ipc(IpcMessage::GetTheme);
        let mut theme = reply.as_object_mut().unwrap().remove("value").unwrap();

        // ensure `powerline_enable` is false
        assert_eq!(
            *theme.pointer("/powerline_enable").unwrap(),
            Value::Bool(false)
        );

        // send message to set it to true
        *theme.pointer_mut("/powerline_enable").unwrap() = Value::Bool(true);
        assert_eq!(
            istat.send_ipc(IpcMessage::SetTheme(theme)),
            json!({ "result": { "detail": null, "type": "success" } })
        );

        // fetch again and assert it was updated
        let reply = istat.send_ipc(IpcMessage::GetTheme);
        let theme = reply.get("value").unwrap();
        assert_eq!(
            *theme.pointer("/powerline_enable").unwrap(),
            Value::Bool(true)
        );
    }
);
