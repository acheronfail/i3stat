set positional-arguments

_default:
  just --list

# setup repository and install dev dependencies
setup:
  if ! command -v cargo-lbuild >/dev/null 2>&1 /dev/null; then cargo install cargo-limit; fi

  if command -v pacman >/dev/null 2>&1 /dev/null; then sudo pacman -S --needed \
    clang dbus dunst libfaketime libpulse i3-wm imagemagick scrot xorg-server-xephyr xorg-server-xvfb yarn; \
  fi

  if command -v apt-get >/dev/null 2>&1 /dev/null; then sudo apt-get install -y \
    build-essential clang dbus dunst i3-wm imagemagick libfaketime libiw-dev libpulse-dev libx11-dev scrot xserver-xephyr xvfb; \
  fi

  if command -v dnf >/dev/null 2>&1 /dev/null; then sudo dnf install -y \
    clang dbus dunst libfaketime i3 ImageMagick iw scrot xorg-x11-server-Xephyr xorg-x11-server-Xvfb libX11-devel yarnpkg; \
  fi

  cd ./scripts/node && yarn

@check +CMDS:
    echo {{CMDS}} | xargs -n1 sh -c 'if ! command -v $1 >/dev/null 2>&1 /dev/null; then echo "$1 is required!"; exit 1; fi' bash

# build the crate
build *args:
  cargo build --all --all-features {{args}}
_lbuild:
  cargo lbuild --all

# runs rustfmt with nightly to enable all its features
fmt:
  rustup run nightly cargo fmt

# run `i3stat` in the terminal and interact with it
dev *args: _lbuild
  cd ./scripts/node && RUST_BACKTRACE=1 RUST_LOG=i3stat=trace yarn dev "$@"

# send an ipc event to the running debug version of i3stat (either `just dev` or `just debug`)
ipc *args: _lbuild
  cargo lrun --quiet --bin i3stat-ipc -- --socket /tmp/i3stat-socket.dev "$@"

# run a binary
run bin *args:
  cargo lrun --bin i3stat-{{bin}} -- "${@:2}"

# install locally, copy sample configuration and restart i3
install *args:
  cargo install --offline --path . "$@"
  mkdir -p ~/.config/i3stat/
  -cp --no-clobber ./sample_config.toml ~/.config/i3stat/config.toml
  i3-msg restart

# start a nested X server with i3 and i3stat
debug dimensions="3800x200": _lbuild
  Xephyr -ac -br -reset -terminate -screen {{dimensions}} :1 &
  until [ -e /tmp/.X11-unix/X1 ]; do sleep 0.1; done
  env -u I3SOCK DISPLAY=:1.0 i3-with-shmlog --config ./scripts/i3.conf

# run tests in a nested dbus session so the host session isn't affected
alias t := test
test *args:
  dbus-run-session -- env RUST_LOG=i3stat=trace I3STAT_TEST=1 cargo test --all "$@"

# `eval` this for an easy debug loop for screenshot tests
# NOTE: requires `fd` be present, and the terminal is `kitty`
@t_screens:
  echo 'function t_screens() {'
  echo '  DEBUG=1 cargo test -- --nocapture screenshots::${1};'
  echo '  for x in `fd --type f "${1}" ./screenshots`; do'
  echo '    echo $x;'
  echo '    kitty +kitten icat $x;'
  echo '  done'
  echo '}'
