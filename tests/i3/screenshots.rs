use serde_json::json;

use crate::i3::X11Test;
use crate::util::Test;

// TODO: use these fake_root mocks in actual tests

macro_rules! screenshot {
    ($name:ident, $item_json:expr) => {
        screenshot!($name, $item_json, {});
    };

    ($name:ident, $item_json:expr, {$($fname:expr => $fdata:expr$(,)?)*}) => {
        x_test!(
            $name,
            {
                // disable separator
                let mut obj = $item_json;
                let map = obj.as_object_mut().unwrap();
                map.insert("separator".into(), false.into());

                // insert item afterwards for some artificial padding
                // done this way because some nerd fonts clip if it's the last item
                json!({ "items": [obj, { "type": "raw", "full_text": "" }] })
            },
            |_test: &mut Test| {
                $(
                    _test.add_fake_file($fname, $fdata);
                )*
            },
            |x_test: X11Test| {
                x_test.screenshot("bar-0");
            }
        );
    }
}

// battery ---------------------------------------------------------------------
// TODO: different states

screenshot! {
    battery,
    json!({
        "type": "battery",
        "interval": "1s"
    }),
    // fake root setup
    {
        "/sys/class/power_supply/BAT0/charge_now" => "7393000",
        "/sys/class/power_supply/BAT0/charge_full" => "7393000",
        "/sys/class/power_supply/BAT0/status" => "Charging",
    }
}

// cpu -------------------------------------------------------------------------
// TODO: mock for tests

screenshot!(
    cpu,
    json!({
        "type": "cpu",
        "interval": "1s"
    })
);

// disk ------------------------------------------------------------------------
// TODO: mock for tests

screenshot!(
    disk,
    json!({
        "type": "disk",
        "interval": "1s"
    })
);

// dunst -----------------------------------------------------------------------
// TODO: mock for tests

screenshot!(dunst, json!({ "type": "dunst" }));

// bar -------------------------------------------------------------------------
// TODO: sample config ?

// screenshot!(bar, json!({}));

// kbd -------------------------------------------------------------------------
// TODO: mock for tests

screenshot!(
    kbd,
    json!({
        "type": "kbd",
        "show": ["caps_lock", "num_lock"]
    })
);

// krb -------------------------------------------------------------------------

screenshot!(
    krb,
    json!({
        "type": "krb",
        "interval": "1s",
    })
);

// mem -------------------------------------------------------------------------
// TODO: mock for tests

screenshot!(
    mem,
    json!({
        "type": "mem",
        "interval": "1s",
    })
);

// net_usage -------------------------------------------------------------------
// TODO: mock for tests
// TODO: pass thresholds for colours

screenshot!(
    net_usage,
    json!({
        "type": "net_usage",
        "interval": "1s",
    })
);

// nic -------------------------------------------------------------------------
// TODO: mock for tests

screenshot!(nic, json!({ "type": "nic" }));

// pulse -----------------------------------------------------------------------
// TODO: mock for tests

screenshot!(pulse, json!({ "type": "pulse" }));

// raw -------------------------------------------------------------------------

screenshot!(
    raw,
    json!({
        "type": "raw",
        "full_text": "Hello, World!",
        "color": "#ff0000",
    })
);

// script ----------------------------------------------------------------------

screenshot!(
    script,
    json!({
        "type": "script",
        "command": "echo -n hello",
        "output": "simple",
    })
);

// sensors ---------------------------------------------------------------------
// TODO: mock for tests

screenshot!(
    sensors,
    json!({
        "type": "sensors",
        "interval": "1s",
        // TODO: use istat-sensors and pick one
        "label": "coretemp Package id 0"
    })
);

// time ------------------------------------------------------------------------

screenshot!(
    time,
    json!({
        "type": "time",
        "interval": "1 s",
        "format_long": "%Y-%m-%d %H:%M:%S",
        "format_short": "%H:%M"
    })
);
