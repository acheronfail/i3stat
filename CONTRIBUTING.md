# Tips

## Nerd Fonts

Nerd Font icons come in two variants. If the font was called "A", the the variants are:

* "A Nerd Font"
* "A Nerd Font Mono"

The one with the "Mono" suffix has all the icons 'squashed' into a single monospace character's width.
This is a compatibility for programs that don't support double-width characters, but does make some of the icons appear too small.

Unfortunately, as far as I can tell, i3's statusbar doesn't properly support the double-width character icons.
So, if the normal font is used, sometimes the icons appear to overlap neighbouring characters.

## Debugging integration tests

The following environment variables are available:

* `DEBUG=1`: increases logs when spawning processes (e.g., `DEBUG=1 cargo test -- --ncapture <test>`)
* `XEPHYR=1`: run X tests with `Xephyr` rather than `Xvfb`

## Why `Rc<str>` over `String`, or `Rc<[T]>` over `Vec<T>` in struct fields?

It's a cheaper method of keeping immutable data around without having to reallocate the inner data every time.
Since mutating the data isn't necessary, this can just point the the existing data and we get cheap clones.
See https://www.youtube.com/watch?v=A4cKi7PTJSs for a good explanation.

# Creating the next release

All new development is done on the `next` branch. When it's time to make a release, a PR is created to `master`.
This is mainly here so I remember how to do a release when I haven't done one in a while.

Steps:

1. Create GH PR from `next` to `master`
2. Push commit with version bump
3. Check CI is green
4. Cargo publish $NEW_VERSION
5. Git tag $NEW_VERSION
6. Update [`i3stat` AUR package](https://aur.archlinux.org/packages/i3stat)
7. Update [`i3stat-bin` AUR package](https://aur.archlinux.org/packages/i3stat-bin)
