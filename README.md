# `staturs`

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
  * Dunst monitor
    * checks if `dunst` has been paused or not, like a "do not disturb" indicator

## Usage

Probably don't use this yet... I'm still hacking on it, but you can have a look around?

## To Do

* [ ] think of a better name than `staturs`
* [ ] handle the `.unwrap()`s
* [ ] all the other "todo"s in the repository
* [ ] publish crate
