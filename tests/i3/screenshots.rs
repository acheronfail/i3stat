use serde_json::json;

use crate::i3::X11Test;
use crate::util::Test;

// TODO: use these fake_root mocks in actual tests

macro_rules! screenshot {
    // shorthand for single case
    ($test_name:ident, $item_json:expr) => {
        screenshot!($test_name, $item_json, {});
    };

    // batch case (many fake_root mocks)
    (
        $test_name:ident,
        $item_json:expr,
        [
            $(
                $case_name:ident => {
                    $($fname:expr => $fdata:expr$(,)?)*
                }$(,)?
            )*
        ]
    ) => {
        $(
            paste::paste! {
                screenshot!([<$test_name _ $case_name>], $item_json, {
                    $($fname => $fdata)*
                });
            }
        )*
    };

    // single case
    (
        $test_name:ident,
        $item_json:expr,
        {
            $($fname:expr => $fdata:expr$(,)?)*
        }
    ) => {
        x_test!(
            $test_name,
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
    };
}

// battery ---------------------------------------------------------------------

screenshot! {
    battery,
    json!({
        "type": "battery",
        "interval": "1s",
        "batteries": [
            "/sys/class/power_supply/BAT0",
            "/sys/class/power_supply/BAT0"
        ],
    }),
    [
        at_100 => {
            "/sys/class/power_supply/BAT0/charge_now" => "100",
            "/sys/class/power_supply/BAT0/charge_full" => "100",
            "/sys/class/power_supply/BAT0/status" => "Discharging",
        },
        at_60 => {
            "/sys/class/power_supply/BAT0/charge_now" => "60",
            "/sys/class/power_supply/BAT0/charge_full" => "100",
            "/sys/class/power_supply/BAT0/status" => "Discharging",
        },
        at_40 => {
            "/sys/class/power_supply/BAT0/charge_now" => "40",
            "/sys/class/power_supply/BAT0/charge_full" => "100",
            "/sys/class/power_supply/BAT0/status" => "Discharging",
        },
        at_20 => {
            "/sys/class/power_supply/BAT0/charge_now" => "20",
            "/sys/class/power_supply/BAT0/charge_full" => "100",
            "/sys/class/power_supply/BAT0/status" => "Discharging",
        },
        at_5 => {
            "/sys/class/power_supply/BAT0/charge_now" => "5",
            "/sys/class/power_supply/BAT0/charge_full" => "100",
            "/sys/class/power_supply/BAT0/status" => "Discharging",
        },
        charging => {
            "/sys/class/power_supply/BAT0/charge_now" => "10",
            "/sys/class/power_supply/BAT0/charge_full" => "100",
            "/sys/class/power_supply/BAT0/status" => "Charging",
        }
        full => {
            "/sys/class/power_supply/BAT0/charge_now" => "100",
            "/sys/class/power_supply/BAT0/charge_full" => "100",
            "/sys/class/power_supply/BAT0/status" => "Full",
        }
    ]
}

// cpu -------------------------------------------------------------------------

screenshot! {
    cpu,
    json!({
        "type": "cpu",
        "interval": "1s",
    }),
    // /proc/stat's values are
    // cpu_id user nice system idle iowait irq softirq steal guest guest_nice
    // for sysinfo's calculations, see: https://github.com/GuillaumeGomez/sysinfo/blob/master/src/linux/cpu.rs
    [
        at_0 => {
            "/proc/stat" => "cpu  0 0 0 0 0 0 0 0 0 0",
        },
        at_50 => {
            "/proc/stat" => "cpu  1 0 0 1 0 0 0 0 0 0",
        },
        at_67 => {
            "/proc/stat" => "cpu  2 0 0 1 0 0 0 0 0 0",
        },
        at_100 => {
            "/proc/stat" => "cpu  1 0 0 0 0 0 0 0 0 0",
        },
    ]
}

// disk ------------------------------------------------------------------------
// TODO: mock for tests

screenshot! {
    disk,
    json!({
        "type": "disk",
        "interval": "1s",
    }),
    [
        // TODO: first checks /proc/mounts, and then uses statvfs
        //  maybe add option to point to disk, and create virtual disk? rather than intercepting statvfs?
    ]
}

// dunst -----------------------------------------------------------------------
// TODO: mock for tests

screenshot!(dunst, json!({ "type": "dunst" }));

// bar -------------------------------------------------------------------------
// TODO: sample config ?

// screenshot!(bar, json!({}));

// kbd -------------------------------------------------------------------------
// TODO: mock for tests

screenshot! {
    kbd,
    json!({
        "type": "kbd",
        "show": ["caps_lock", "num_lock"]
    }),
    [
        caps_on => {
            // FIXME: need to be able to mock a read_dir request
            // FIXME: or alternatively, be able to match a glob?
            "/sys/class/leds/input0::capslock/brightness" => "1",
            "/sys/class/leds/input0::numlock/brightness" => "0",
        }
    ]
}

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
