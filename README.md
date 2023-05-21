# `istat`

I used to use [`i3blocks`](https://github.com/vivien/i3blocks) for `i3`'s `status_command`, but I found that having all
my configuration in separate scripts was getting a little cumbersome.

That, and also I could never find a good block for volume control that wasn't janky or slow.

So, I decided to write my own `status_command` generator, and what better language to write it in than Rust!

## Features

* completely single threaded
  * less resource usage - it's a status command, it shouldn't be heavy
* many different integrations
  * PulseAudio (works with pipewire)
  * Disk space
  * Time
  * Memory usage
  * CPU usage
  * CPU temperature
  * Network interface + status (IP addresses, wifi ssid and quality, etc)
  * Network usage (active monitoring for upload/download)
  * Arbitrary scripts
  * Kerberos status
  * Dunst monitor
    * checks if `dunst` has been paused or not, like a "do not disturb" indicator
* ipc control
  * send click events via a command
  * custom events for some integrations (e.g., controlling PulseAudio, etc)
  * refresh items

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
