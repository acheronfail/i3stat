use serde_json::json;

use crate::i3::X11Test;
use crate::util::Test;

// TODO: use these fake_root mocks in actual tests
//  ie: re-use these in `spawn` tests and check json

macro_rules! screenshot {
    // shorthand when mocks aren't needed
    (
        $test_name:ident,
        $item_json:expr
    ) => {
        screenshot!($test_name, $item_json, { default: {} });
    };

    // batch case (many cases with bin/fake_root mocks)
    (
        $test_name:ident,
        $item_json:expr
        $(,
            {
                $(
                    $case_name:ident: {
                        $(bin => $bname:literal: $bdata:expr,)*
                        $(file => $fname:literal: $fdata:expr,)*
                        $(fn => $extra:expr)?
                    }$(,)?
                )+
            }
        )+
    ) => {
        $(
            $(
                paste::paste! {
                    screenshot!(
                        [<$test_name _ $case_name>],
                        $item_json,
                        bins = {
                            $($bname => $bdata)*
                        },
                        roots = {
                            $($fname => $fdata)*
                        }
                        $(, $extra)?
                    );
                }
            )*
        )?
    };

    // single case
    (
        $test_name:ident,
        $item_json:expr,
        bins = {
            $($bname:expr => $bdata:expr$(,)?)*
        },
        roots = {
            $($fname:expr => $fdata:expr$(,)?)*
        }
        $(, $extra:expr)?
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
                    _test.add_bin($bname, $bdata);
                )*
                $(
                    _test.add_fake_file($fname, $fdata);
                )*
            },
            |x_test: X11Test| {
                $($extra(&x_test);)?
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
    {
        at_100: {
            file => "/sys/class/power_supply/BAT0/charge_now": "100",
            file => "/sys/class/power_supply/BAT0/charge_full": "100",
            file => "/sys/class/power_supply/BAT0/status": "Discharging",
        },
        at_60: {
            file => "/sys/class/power_supply/BAT0/charge_now": "60",
            file => "/sys/class/power_supply/BAT0/charge_full": "100",
            file => "/sys/class/power_supply/BAT0/status": "Discharging",
        },
        at_40: {
            file => "/sys/class/power_supply/BAT0/charge_now": "40",
            file => "/sys/class/power_supply/BAT0/charge_full": "100",
            file => "/sys/class/power_supply/BAT0/status": "Discharging",
        },
        at_20: {
            file => "/sys/class/power_supply/BAT0/charge_now": "20",
            file => "/sys/class/power_supply/BAT0/charge_full": "100",
            file => "/sys/class/power_supply/BAT0/status": "Discharging",
        },
        at_5: {
            file => "/sys/class/power_supply/BAT0/charge_now": "5",
            file => "/sys/class/power_supply/BAT0/charge_full": "100",
            file => "/sys/class/power_supply/BAT0/status": "Discharging",
        },
        charging: {
            file => "/sys/class/power_supply/BAT0/charge_now": "10",
            file => "/sys/class/power_supply/BAT0/charge_full": "100",
            file => "/sys/class/power_supply/BAT0/status": "Charging",
        }
        full: {
            file => "/sys/class/power_supply/BAT0/charge_now": "100",
            file => "/sys/class/power_supply/BAT0/charge_full": "100",
            file => "/sys/class/power_supply/BAT0/status": "Full",
        }
    }
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
    // for sysinfo's calculations:
    // see: https://github.com/GuillaumeGomez/sysinfo/blob/2fa03b052e92f4d8ce90e57c548b1732f848dbbd/src/linux/cpu.rs
    {
        at_0: {
            file => "/proc/stat": "cpu  0 0 0 0 0 0 0 0 0 0",
        },
        at_50: {
            file => "/proc/stat": "cpu  1 0 0 1 0 0 0 0 0 0",
        },
        at_67: {
            file => "/proc/stat": "cpu  2 0 0 1 0 0 0 0 0 0",
        },
        at_100: {
            file => "/proc/stat": "cpu  1 0 0 0 0 0 0 0 0 0",
        },
    }
}

// disk ------------------------------------------------------------------------
// TODO: mock for tests

screenshot! {
    disk,
    json!({
        "type": "disk",
        "interval": "1s",
    }),
    {
        todo: {}
        // TODO: first checks /proc/mounts, and then uses statvfs
        //  maybe add option to point to disk, and create virtual disk? rather than intercepting statvfs?
    }
}

// dunst -----------------------------------------------------------------------

screenshot!(
    dunst,
    json!({ "type": "dunst" }),
    {
        off: {},
        on: { fn => |t: &X11Test| t.i3_msg("exec 'dunstctl set-paused true'") }
    }
);

// bar -------------------------------------------------------------------------
// TODO: sample config ?
// TODO: powerline examples ?
//  will need to change screenshot size

// screenshot!(bar, json!({}));

// kbd -------------------------------------------------------------------------

screenshot! {
    kbd,
    json!({
        "type": "kbd",
        "show": ["caps_lock", "num_lock", "scroll_lock"]
    }),
    {
        caps_on: {
            file => "/sys/class/leds/input0::capslock/brightness": "1",
            file => "/sys/class/leds/input0::numlock/brightness": "0",
            file => "/sys/class/leds/input0::scrolllock/brightness": "0",
        },
        num_on: {
            file => "/sys/class/leds/input0::capslock/brightness": "0",
            file => "/sys/class/leds/input0::numlock/brightness": "1",
            file => "/sys/class/leds/input0::scrolllock/brightness": "0",
        },
        all_on: {
            file => "/sys/class/leds/input0::capslock/brightness": "1",
            file => "/sys/class/leds/input0::numlock/brightness": "1",
            file => "/sys/class/leds/input0::scrolllock/brightness": "1",
        },
        all_off: {
            file => "/sys/class/leds/input0::capslock/brightness": "0",
            file => "/sys/class/leds/input0::numlock/brightness": "0",
            file => "/sys/class/leds/input0::scrolllock/brightness": "0",
        },
        one_err: {
            file => "/sys/class/leds/input0::capslock/brightness": "1",
            file => "/sys/class/leds/input0::numlock/brightness": "0",
        }
    }
}

// krb -------------------------------------------------------------------------

screenshot!(
    krb,
    json!({
        "type": "krb",
        "interval": "1s",
    }),
    {
        off: {
            bin => "klist": "#!/usr/bin/env bash\nexit 0",
        },
        on: {
            bin => "klist": "#!/usr/bin/env bash\nexit 1",
        }
    }
);

// mem -------------------------------------------------------------------------

fn mem(total: u64, available: u64) -> String {
    format!(
        r#"\
MemTotal:       {total} kB
MemFree:              0 kB
MemAvailable:   {available} kB
Buffers:              0 kB
Cached:               0 kB
Shmem:                0 kB
SReclaimable:         0 kB
SwapTotal:      {total} kB
SwapFree:       {total} kB"#,
        total = total,
        available = available
    )
}

screenshot!(
    mem,
    json!({
        "type": "mem",
        "interval": "1s",
    }),
    // for sysinfo calculations:
    // see: https://github.com/GuillaumeGomez/sysinfo/blob/2fa03b052e92f4d8ce90e57c548b1732f848dbbd/src/linux/system.rs#L257
    {
        free_100: { file => "/proc/meminfo": mem(31250000, 31250000), },
        free_75: { file => "/proc/meminfo": mem(31250000, 23437500), },
        free_50: { file => "/proc/meminfo": mem(31250000, 15625000), },
        free_25: { file => "/proc/meminfo": mem(31250000, 7812500), },
        free_0: { file => "/proc/meminfo": mem(31250000, 0), },

        at_0: {
            file => "/proc/meminfo": mem(31250000, 31250000),
            fn => |test: &X11Test| test.istat_ipc("click mem left")
        },
        at_25: {
            file => "/proc/meminfo": mem(31250000, 23437500),
            fn => |test: &X11Test| test.istat_ipc("click mem left")
        },
        at_50: {
            file => "/proc/meminfo": mem(31250000, 15625000),
            fn => |test: &X11Test| test.istat_ipc("click mem left")
        },
        at_75: {
            file => "/proc/meminfo": mem(31250000, 7812500),
            fn => |test: &X11Test| test.istat_ipc("click mem left")
        },
        at_100: {
            file => "/proc/meminfo": mem(31250000, 0),
            fn => |test: &X11Test| test.istat_ipc("click mem left")
         }
    }
);

// net_usage -------------------------------------------------------------------

screenshot!(
    net_usage,
    json!({
        "type": "net_usage",
        "interval": "1s",
        "minimum": "1B",
        "display": "bits", // after click it turns to bytes for screenshot
        "thresholds": ["1kiB", "1MiB", "10MiB", "25MiB", "100MiB"],
        // this is the least hacky solution I can think of right now...
        "_always_assume_interval": true
    }),
    // Mocking these out is tricky, since it's a constant measure of bytes over time.
    // So we start at 0, add some bytes, and then click to check again.
    // https://github.com/GuillaumeGomez/sysinfo/blob/2fa03b052e92f4d8ce90e57c548b1732f848dbbd/src/linux/network.rs#L53
    {
        no_traffic: {
            file => "/sys/class/net/wlan0/statistics/rx_bytes": "0",
            file => "/sys/class/net/wlan0/statistics/tx_bytes": "0",
        },
        threshold_0: {
            file => "/sys/class/net/wlan1/statistics/rx_bytes": "0",
            file => "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            fn => |t: &X11Test| {
                t.cmd("echo 1 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 2 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            }
        },
        threshold_1: {
            file => "/sys/class/net/wlan1/statistics/rx_bytes": "0",
            file => "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            fn => |t: &X11Test| {
                t.cmd("echo 2048 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 4096 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            }
        },
        threshold_2: {
            file => "/sys/class/net/wlan1/statistics/rx_bytes": "0",
            file => "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            fn => |t: &X11Test| {
                t.cmd("echo 4000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 8000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            }
        },
        threshold_3: {
            file => "/sys/class/net/wlan1/statistics/rx_bytes": "0",
            file => "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            fn => |t: &X11Test| {
                t.cmd("echo 14000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 18000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            }
        },
        threshold_4: {
            file => "/sys/class/net/wlan1/statistics/rx_bytes": "0",
            file => "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            fn => |t: &X11Test| {
                t.cmd("echo 31000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 32000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            }
        },
        threshold_max: {
            file => "/sys/class/net/wlan1/statistics/rx_bytes": "0",
            file => "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            fn => |t: &X11Test| {
                t.cmd("echo 420000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 430000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            }
        },
    }
);

// nic -------------------------------------------------------------------------
// TODO: mock (getifaddrs?) for tests

screenshot!(nic, json!({ "type": "nic" }));

// pulse -----------------------------------------------------------------------
// TODO: mock (pulse/pipewire?) for tests

screenshot!(pulse, json!({ "type": "pulse" }));

// raw -------------------------------------------------------------------------

screenshot!(
    raw,
    json!({
        "type": "raw",
        "full_text": "Hello, World!",
        "background": "#00ff00",
        "color": "#ff00ff",
        "border": "#ff00ff"
    })
);

// script ----------------------------------------------------------------------

screenshot!(
    script,
    json!({
        "type": "script",
        "command": "echo -n hello",
        "output": "simple",
    }),
    {todo: {}}
);

// sensors ---------------------------------------------------------------------

screenshot!(
    sensors,
    json!({
        "type": "sensors",
        "interval": "1s",
        "label": "name temp1"
    }),
    {
        at_0: {
            file => "/sys/class/hwmon/hwmon1/name": "name",
            file => "/sys/class/hwmon/hwmon1/temp1_input": "0",
        },
        at_50: {
            file => "/sys/class/hwmon/hwmon1/name": "name",
            file => "/sys/class/hwmon/hwmon1/temp1_input": "50000",
        },
        at_70: {
            file => "/sys/class/hwmon/hwmon1/name": "name",
            file => "/sys/class/hwmon/hwmon1/temp1_input": "70000",
        },
        at_80: {
            file => "/sys/class/hwmon/hwmon1/name": "name",
            file => "/sys/class/hwmon/hwmon1/temp1_input": "80000",
        },
        at_100: {
            file => "/sys/class/hwmon/hwmon1/name": "name",
            file => "/sys/class/hwmon/hwmon1/temp1_input": "100000",
        }
    }
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
