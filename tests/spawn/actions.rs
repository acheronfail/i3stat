use istat::i3::{I3Button, I3Modifier};
use serde_json::json;

use crate::spawn::SpawnedProgram;
use crate::util::Test;

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
                "#!/usr/bin/env bash\necho -n ${{0##*/}} > /out; echo $PATH; which foo; istat-ipc --socket {} signal 0",
                test.istat_socket_file.display()
            )
        };

        test.add_bin("foo", echo_name_then_signal());
        test.add_bin("bar", echo_name_then_signal());
        test.add_bin("baz", echo_name_then_signal());
    },
    |mut istat: SpawnedProgram| {
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "asdf" }])
        );

        // FIXME: https://github.com/acheronfail/istat/actions/runs/6521800730/job/17710733529?pr=11
        // in CI it looks like it can't find "foo" command
        istat.click("0", I3Button::Left, &[]);
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "foo" }])
        );

        istat.click("0", I3Button::Middle, &[I3Modifier::Shift]);
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "bar" }])
        );

        istat.click("0", I3Button::Right, &[I3Modifier::Control]);
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "baz" }])
        );
        istat.click("0", I3Button::Right, &[I3Modifier::Shift]);
        assert_eq!(
            istat.next_line_json().unwrap(),
            json!([{ "instance": "0", "name": "script", "full_text": "foo" }])
        );
    }
);
