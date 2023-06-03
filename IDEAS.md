# Ideas

This document is more or less a scratchpad for ideas of things to work on next.
There's no guarantee they'll ever be added or implemented, and they'll likely be half-baked!

## Features

* conditionally include additional config files
  * i.e., different machines
* conditionally disable bar items
* a bin PKGBUILD for the AUR (would need to setup CI first)

## Bugs

* ...

## Improvements

* script to generate and resize screenshots to easily update readme
  * `scrot` + `convert` with `Xephyr`, etc
* refactor `Context::paginate` into a wrapping item or something with a more well-defined API - right now it's not nice to use

## Tips

### Nerd Fonts

Nerd Font icons come in two variants. If the font was called "A", the the variants are:

* "A Nerd Font"
* "A Nerd Font Mono"

The one with the "Mono" suffix has all the icons 'squashed' into a single monospace character's width.
This is a compatibility for programs that don't support double-width characters, but does make some of the icons appear too small.
