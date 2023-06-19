# Ideas

This document is more or less a scratchpad for ideas of things to work on next.
There's no guarantee they'll ever be added or implemented, and they'll likely be half-baked!

## Features

* conditionally include additional config files
  * i.e., different machines
  * how to modify order of items across config files?
    * `item_order` is concatenated, and this isn't very intuitive
    * add something to `Common`?
      * `index = n`
        * if item exists at `n`, push to right
        * what if it's past the end? shrink down?
        * does it make sense if it's defined in the main config file?
        * multiple items of same index, then in iteration order perform sort
* conditionally disable bar items
* a bin PKGBUILD for the AUR (would need to setup CI first)
* man pages for all binaries

## Bugs

* ...

## Improvements

* script to generate and resize screenshots to easily update readme
  * `scrot` + `convert` with `Xephyr`, etc

## Tips

### Nerd Fonts

Nerd Font icons come in two variants. If the font was called "A", the the variants are:

* "A Nerd Font"
* "A Nerd Font Mono"

The one with the "Mono" suffix has all the icons 'squashed' into a single monospace character's width.
This is a compatibility for programs that don't support double-width characters, but does make some of the icons appear too small.

Unfortunately, as far as I can tell, i3's statusbar doesn't properly support the double-width character icons.
So, if the normal font is used, sometimes the icons appear to overlap neighbouring characters.

### Debugging integration tests

The following environment variables are available:

* `DEBUG=1`: increases logs when spawning processes (e.g., `DEBUG=1 cargo test -- --ncapture <test>`)
* `XEPHYR=1`: run X tests with `Xephyr` rather than `Xvfb`

### Why `Rc<str>` over `String`, or `Rc<[T]>` over `Vec<T>` in struct fields?

It's a cheaper method of keeping immutable data around without having to reallocate the inner data every time.
Since mutating the data isn't necessary, this can just point the the exiting data and we get cheap clones.
See https://www.youtube.com/watch?v=A4cKi7PTJSs for a good explanation.
