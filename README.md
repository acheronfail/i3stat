# `istat`: an i3 (or sway) status_command

I used to use [`i3blocks`](https://github.com/vivien/i3blocks) for `i3`'s `status_command`, but I found that having all
my configuration in separate scripts was getting a little cumbersome.

That, and also I could never find a good block for volume control that wasn't janky or slow.

So, I decided to write my own `status_command` generator, and what better language to write it in than Rust!

## Features

* completely single threaded
  * less resource usage - it's a status command, it shouldn't be heavy
* ipc control
  * send click events via a command
  * refresh items with a command
  * custom events for some integrations (e.g., controlling PulseAudio, etc)
* many different bar items (continue reading for screenshots)

Each bar item is configurable, see [the sample config](./sample_config.toml) for options.

Here's a short demo with some screenshots:

![](./.github/assets/full.png)

**battery:** percentage, charging, etc. Supports multiple batteries.

<div style="text-align:center;"><img src="./.github/assets/battery_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/battery_2.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/battery_3.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/battery_4.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/battery_5.png" width="200" /></div>

**cpu:** usage expressed as a percentage

<div style="text-align:center;"><img src="./.github/assets/cpu_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/cpu_2.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/cpu_3.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/cpu_4.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/cpu_5.png" width="200" /></div>

**disk:** usage, shows free disk space

<div style="text-align:center;"><img src="./.github/assets/disk_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/disk_2.png" width="200" /></div>

**dunst:** displays "do not disturb" status (if it's paused or not)

<div style="text-align:center;"><img src="./.github/assets/dunst_1.png" width="200" /></div>

**kbd:** displays CapsLock/Numlock/etc states

<div style="text-align:center;"><img src="./.github/assets/kbd_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/kbd_2.png" width="200" /></div>

**krb:** checks if a valid kerberos token exists (like `klist -s`)

<div style="text-align:center;"><img src="./.github/assets/krb_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/krb_2.png" width="200" /></div>

**mem:** display free memory as bytes or as a percentage

<div style="text-align:center;"><img src="./.github/assets/mem_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/mem_2.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/mem_3.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/mem_4.png" width="200" /></div>

**net_speed:** upload and download statistics

<div style="text-align:center;"><img src="./.github/assets/net_speed_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/net_speed_2.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/net_speed_3.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/net_speed_4.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/net_speed_5.png" width="200" /></div>

**nic:** network interface status - connection state and ip addresses

<div style="text-align:center;"><img src="./.github/assets/nic_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/nic_2.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/nic_3.png" width="200" /></div>

**pulse:** input/output volume status, control and connected speaker type

<div style="text-align:center;"><img src="./.github/assets/pulse_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/pulse_2.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/pulse_3.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/pulse_4.png" width="200" /></div>

**script:** run arbitrary scripts and show their output

<div style="text-align:center;"><img src="./.github/assets/script_1.png" width="200" /></div>

**sensors:** temperature sensors

<div style="text-align:center;"><img src="./.github/assets/sensors_1.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/sensors_2.png" width="200" /></div>
<div style="text-align:center;"><img src="./.github/assets/sensors_3.png" width="200" /></div>

**time:** displays the current date and/or time

<div style="text-align:center;"><img src="./.github/assets/time_1.png" width="200" /></div>


## Install

With Rust (via cargo):

```sh
cargo install istat
# Make sure to look at the `sample_config.toml` file for configuration options
```

Via the AUR (Arch Linux):

```sh
paru -S istat
```

## Usage

### Setting it up

First, create a config file for `istat`. View [the sample config](./sample_config.toml) for what's available.
This file should be placed in:

* `$XDG_CONFIG_HOME/istat/<here>`, or
* `$HOME/.config/istat/<here>`

Even though the [sample configuration file](./sample_config.toml) is a TOML file, YAML and JSON are also supported.

Then, update your i3/sway config to use `istat` as the `status_command`:

```
bar {
        status_command istat
        # ... other config
}
```

### Interacting with `istat`

`istat` offers multiple ways of interacting with it:

* standard click events from i3/sway
* real-time signals
* it's own ipc

#### Signals

Consider the following bar item which outputs the state of the CapsLock and NumLock keys:

```toml
type = "kbd"
show = ["caps_lock", "num_lock"]
interval = "30s"
```

It refreshes every 30 seconds, or every time the bar item receives a click event. That's alright, but we can do better with signals.
Adding `signal = 8` to the config, and removing `interval` we get:

```toml
type = "kbd"
show = ["caps_lock", "num_lock"]
signal = 8
```

Now, whenever `istat` receive the `SIGRTMIN+8` signal, the bar item will be refreshed.
Pair this with the following config in i3/sway, and you'll have a bar item that reflects your keys all the time:

```
bindsym --release Caps_Lock exec --no-startup-id pkill -RTMIN+8 istat
bindsym --release Num_Lock  exec --no-startup-id pkill -RTMIN+8 istat
```

Linux offers many realtime signals, to see which your machine supports the `istat-signals` command is provided:

```bash
$ istat-signals
{"max":30,"min":0,"sigrtmax":64,"sigrtmin":34}
```

The same signal can be configured for multiple bar items, too!

#### Custom IPC events

The command `istat-ipc` is provided to interface with `istat`. It supports:

* fetching the name and index of all the currently running bar items
* refreshing all bar items at once
* sending `click` events to each bar item
* sending custom events to bar items
  * some bar items (like `pulse`) expose an advanced API which can be accessed with these events

**Refresh all bar items at once**:

```bash
istat-ipc refresh-all
```

**Send a click event to a bar item - without actually clicking it!**:

```bash
# emulate a left click on the disk item:
istat-ipc click disk left
```

**Control PulseAudio/Pipewire via custom IPC events**:

```bash
# see all the custom events that pulse has to offer:
istat-ipc custom pulse

# Some examples:

# turn the output (speakers) volume up
istat-ipc custom pulse volume-down sink
# turn the input (microphone) volume down
istat-ipc custom pulse volume-up   source
# mute or unmute the output
istat-ipc custom pulse mute-toggle sink
```
