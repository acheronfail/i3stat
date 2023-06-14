# Ideas

This document is more or less a scratchpad for ideas of things to work on next.
There's no guarantee they'll ever be added or implemented, and they'll likely be half-baked!

## Features

* conditionally include additional config files
  * i.e., different machines
* conditionally disable bar items
* a bin PKGBUILD for the AUR (would need to setup CI first)
* man pages for all binaries

## Bugs

* restarting pipewire/pulseaudio breaks pulse item

## Improvements

* script to generate and resize screenshots to easily update readme
  * `scrot` + `convert` with `Xephyr`, etc
* tests
  * unit tests for what makes sense
  * xephyr tests for i3 interactions
  * ipc tests

## Tips

### Nerd Fonts

Nerd Font icons come in two variants. If the font was called "A", the the variants are:

* "A Nerd Font"
* "A Nerd Font Mono"

The one with the "Mono" suffix has all the icons 'squashed' into a single monospace character's width.
This is a compatibility for programs that don't support double-width characters, but does make some of the icons appear too small.

Unfortunately, as far as I can tell, i3's statusbar doesn't properly support the double-width character icons.
So, if the normal font is used, sometimes the icons appear to overlap neighbouring characters.