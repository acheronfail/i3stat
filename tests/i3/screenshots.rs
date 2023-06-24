use serde_json::json;

use crate::i3::X11Test;
use crate::util::Test;

macro_rules! screenshot {
    // shorthand when mocks aren't needed
    (
        $(@$dbus:ident$(,)?)?
        $test_name:ident,
        $item_json:expr
    ) => {
        screenshot!($(@$dbus,)? $test_name, $item_json, { default: {} });
    };

    // batch case (many cases with bin/fake_root mocks)
    (
        $test_name:ident,
        $item_json:expr
        $(,
            {
                $(
                    $case_name:ident: {
                        $(@$dbus:ident$(,)?)?
                        $(bins => {$($bname:literal: $bdata:expr$(,)?)*};)?
                        $(files => {$($fname:literal: $fdata:expr$(,)?)*};)?
                        $(setup_fn => $setup_fn:expr;)?
                        $(test_fn => $test_fn:expr;)?
                    }$(,)?
                )+
            }
        )+
    ) => {
        $(
            $(
                paste::paste! {
                    screenshot!(
                        $(@$dbus,)?
                        [<$test_name _ $case_name>],
                        $item_json,
                        bins = {
                            $($($bname => $bdata)*)?
                        },
                        roots = {
                            $($($fname => $fdata)*)?
                        }
                        $(, setup_fn => $setup_fn)?
                        $(, test_fn => $test_fn)?
                    );
                }
            )*
        )?
    };

    // single case
    (
        $(@$dbus:ident$(,)?)?
        $test_name:ident,
        $item_json:expr,
        bins = {
            $($bname:expr => $bdata:expr$(,)?)*
        },
        roots = {
            $($fname:expr => $fdata:expr$(,)?)*
        }
        $(, setup_fn => $setup_fn:expr)?
        $(, test_fn => $test_fn:expr)?
    ) => {
        x_test!(
            $(@$dbus)?
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
                $($setup_fn(&_test);)?
                $(
                    _test.add_bin($bname, $bdata);
                )*
                $(
                    _test.add_fake_file($fname, $fdata);
                )*
            },
            |x_test: &X11Test| {
                $($test_fn(&x_test);)?
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
            files => {
                "/sys/class/power_supply/BAT0/charge_now": "100",
                "/sys/class/power_supply/BAT0/charge_full": "100",
                "/sys/class/power_supply/BAT0/status": "Discharging",
            };
        },
        at_60: {
            files => {
                "/sys/class/power_supply/BAT0/charge_now": "60",
                "/sys/class/power_supply/BAT0/charge_full": "100",
                "/sys/class/power_supply/BAT0/status": "Discharging",
            };
        },
        at_40: {
            files => {
                "/sys/class/power_supply/BAT0/charge_now": "40",
                "/sys/class/power_supply/BAT0/charge_full": "100",
                "/sys/class/power_supply/BAT0/status": "Discharging",
            };
        },
        at_20: {
            files => {
                "/sys/class/power_supply/BAT0/charge_now": "20",
                "/sys/class/power_supply/BAT0/charge_full": "100",
                "/sys/class/power_supply/BAT0/status": "Discharging",
            };
        },
        at_5: {
            files => {
                "/sys/class/power_supply/BAT0/charge_now": "5",
                "/sys/class/power_supply/BAT0/charge_full": "100",
                "/sys/class/power_supply/BAT0/status": "Discharging",
            };
        },
        charging: {
            files => {
                "/sys/class/power_supply/BAT0/charge_now": "10",
                "/sys/class/power_supply/BAT0/charge_full": "100",
                "/sys/class/power_supply/BAT0/status": "Charging",
            };
        }
        full: {
            files => {
                "/sys/class/power_supply/BAT0/charge_now": "100",
                "/sys/class/power_supply/BAT0/charge_full": "100",
                "/sys/class/power_supply/BAT0/status": "Full",
            };
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
            files => { "/proc/stat": "cpu  0 0 0 0 0 0 0 0 0 0" };
        },
        at_50: {
            files => { "/proc/stat": "cpu  1 0 0 1 0 0 0 0 0 0" };
        },
        at_67: {
            files => { "/proc/stat": "cpu  2 0 0 1 0 0 0 0 0 0" };
        },
        at_100: {
            files => { "/proc/stat": "cpu  1 0 0 0 0 0 0 0 0 0" };
        },
    }
}

// disk ------------------------------------------------------------------------

// NOTE: this one is difficult to mock, since it first reads `/proc/mount` and then
// proceeds to call `statvfs` to get filesystem information.
screenshot! {
    disk,
    json!({
        "type": "disk",
        "interval": "1s",
    })
}

// dunst -----------------------------------------------------------------------

// FIXME: ensure this is run with `dbus-run-session` so it doesn't interfere with host
//  or some alternative
screenshot!(
    dunst,
    json!({ "type": "dunst" }),
    {
        off: { @dbus },
        on: { @dbus, test_fn => |t: &X11Test| t.cmd("i3-msg exec 'dunstctl set-paused true'"); }
    }
);

// kbd -------------------------------------------------------------------------

screenshot! {
    kbd,
    json!({
        "type": "kbd",
        "show": ["caps_lock", "num_lock", "scroll_lock"]
    }),
    {
        caps_on: {
            files => {
                "/sys/class/leds/input0::capslock/brightness": "1",
                "/sys/class/leds/input0::numlock/brightness": "0",
                "/sys/class/leds/input0::scrolllock/brightness": "0",
            };
        },
        num_on: {
            files => {
                "/sys/class/leds/input0::capslock/brightness": "0",
                "/sys/class/leds/input0::numlock/brightness": "1",
                "/sys/class/leds/input0::scrolllock/brightness": "0",
            };
        },
        all_on: {
            files => {
                "/sys/class/leds/input0::capslock/brightness": "1",
                "/sys/class/leds/input0::numlock/brightness": "1",
                "/sys/class/leds/input0::scrolllock/brightness": "1",
            };
        },
        all_off: {
            files => {
                "/sys/class/leds/input0::capslock/brightness": "0",
                "/sys/class/leds/input0::numlock/brightness": "0",
                "/sys/class/leds/input0::scrolllock/brightness": "0",
            };
        },
        one_err: {
            files => {
                "/sys/class/leds/input0::capslock/brightness": "1",
                "/sys/class/leds/input0::numlock/brightness": "0",
            };
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
        on: {
            bins => { "klist": "#!/usr/bin/env bash\nexit 0" };
        },
        off: {
            bins => { "klist": "#!/usr/bin/env bash\nexit 1" };
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
        free_100: { files => { "/proc/meminfo": mem(31250000, 31250000) }; },
        free_75: { files => { "/proc/meminfo": mem(31250000, 23437500) }; },
        free_50: { files => { "/proc/meminfo": mem(31250000, 15625000) }; },
        free_25: { files => { "/proc/meminfo": mem(31250000, 7812500) }; },
        free_0: { files => { "/proc/meminfo": mem(31250000, 0) }; },

        at_0: {
            files => { "/proc/meminfo": mem(31250000, 31250000) };
            test_fn => |test: &X11Test| test.istat_ipc("click mem left");
        },
        at_25: {
            files => { "/proc/meminfo": mem(31250000, 23437500) };
            test_fn => |test: &X11Test| test.istat_ipc("click mem left");
        },
        at_50: {
            files => { "/proc/meminfo": mem(31250000, 15625000) };
            test_fn => |test: &X11Test| test.istat_ipc("click mem left");
        },
        at_75: {
            files => { "/proc/meminfo": mem(31250000, 7812500) };
            test_fn => |test: &X11Test| test.istat_ipc("click mem left");
        },
        at_100: {
            files => { "/proc/meminfo": mem(31250000, 0) };
            test_fn => |test: &X11Test| test.istat_ipc("click mem left");
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
            files => {
                "/sys/class/net/wlan0/statistics/rx_bytes": "0",
                "/sys/class/net/wlan0/statistics/tx_bytes": "0",
            };
        },
        threshold_0: {
            files => {
                "/sys/class/net/wlan1/statistics/rx_bytes": "0",
                "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            };
            test_fn => |t: &X11Test| {
                t.cmd("echo 1 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 2 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            };
        },
        threshold_1: {
            files => {
                "/sys/class/net/wlan1/statistics/rx_bytes": "0",
                "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            };
            test_fn => |t: &X11Test| {
                t.cmd("echo 2048 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 4096 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            };
        },
        threshold_2: {
            files => {
                "/sys/class/net/wlan1/statistics/rx_bytes": "0",
                "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            };
            test_fn => |t: &X11Test| {
                t.cmd("echo 4000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 8000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            };
        },
        threshold_3: {
            files => {
                "/sys/class/net/wlan1/statistics/rx_bytes": "0",
                "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            };
            test_fn => |t: &X11Test| {
                t.cmd("echo 14000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 18000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            };
        },
        threshold_4: {
            files => {
                "/sys/class/net/wlan1/statistics/rx_bytes": "0",
                "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            };
            test_fn => |t: &X11Test| {
                t.cmd("echo 31000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 32000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            };
        },
        threshold_max: {
            files => {
                "/sys/class/net/wlan1/statistics/rx_bytes": "0",
                "/sys/class/net/wlan1/statistics/tx_bytes": "0",
            };
            test_fn => |t: &X11Test| {
                t.cmd("echo 420000000 > /sys/class/net/wlan1/statistics/rx_bytes");
                t.cmd("echo 430000000 > /sys/class/net/wlan1/statistics/tx_bytes");
                t.istat_ipc("click net_usage left");
            };
        },
    }
);

// nic -------------------------------------------------------------------------

// NOTE: this is difficult to mock, since it uses `getifaddrs` for interface information
// and then also uses `iwlib` to read WiFi information
screenshot!(nic, json!({ "type": "nic" }));

// pulse -----------------------------------------------------------------------

// NOTE: I'm not about to spin up a pulse server just for a test...
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
    })
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
            files => {
                "/sys/class/hwmon/hwmon1/name": "name",
                "/sys/class/hwmon/hwmon1/temp1_input": "0",
            };
        },
        at_50: {
            files => {
                "/sys/class/hwmon/hwmon1/name": "name",
                "/sys/class/hwmon/hwmon1/temp1_input": "50000",
            };
        },
        at_70: {
            files => {
                "/sys/class/hwmon/hwmon1/name": "name",
                "/sys/class/hwmon/hwmon1/temp1_input": "70000",
            };
        },
        at_80: {
            files => {
                "/sys/class/hwmon/hwmon1/name": "name",
                "/sys/class/hwmon/hwmon1/temp1_input": "80000",
            };
        },
        at_100: {
            files => {
                "/sys/class/hwmon/hwmon1/name": "name",
                "/sys/class/hwmon/hwmon1/temp1_input": "100000",
            };
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
