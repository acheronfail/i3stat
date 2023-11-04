use i3stat::i3::{I3Button, I3Modifier};
use serde_json::json;

use crate::spawn::SpawnedProgram;
use crate::util::{get_exe, Test};

spawn_test!(
    actions,
    json!({
      "items": [
        {
          "type": "script",
          "command": "cat /out",
          "actions": {
            "left_click": "foo",
            "middle_click": { "modifiers": ["Shift"], "command": "bar" },
            "right_click": [
                { "modifiers": ["Control"], "command": "baz" },
                { "modifiers": ["Shift"], "command": "foo" },
            ]
          }
        }
      ]
    }),
    |test: &mut Test| {
        test.add_fake_file("out", "asdf");
        let echo_name_then_signal = || {
            format!(
                "#!/usr/bin/env bash\necho -n ${{0##*/}} > /out; {ipc} --socket {socket} signal 0",
                ipc = get_exe("i3stat-ipc").display(),
                socket = test.i3stat_socket_file.display()
            )
        };

        test.add_bin("foo", echo_name_then_signal());
        test.add_bin("bar", echo_name_then_signal());
        test.add_bin("baz", echo_name_then_signal());
    },
    |mut i3stat: SpawnedProgram| {
        assert_eq!(
            i3stat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "asdf" }])
        );

        i3stat.click("0", I3Button::Left, &[]);
        assert_eq!(
            i3stat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "foo" }])
        );

        i3stat.click("0", I3Button::Middle, &[I3Modifier::Shift]);
        assert_eq!(
            i3stat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "bar" }])
        );

        i3stat.click("0", I3Button::Right, &[I3Modifier::Control]);
        assert_eq!(
            i3stat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "baz" }])
        );
        i3stat.click("0", I3Button::Right, &[I3Modifier::Shift]);
        assert_eq!(
            i3stat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "foo" }])
        );
    }
);
