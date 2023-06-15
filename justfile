set positional-arguments

_default:
  just --list

# setup repository and install dev dependencies
setup:
  cd ./scripts/run && yarn
  if ! command -v cargo-lbuild >/dev/null 2>&1 /dev/null; then cargo install cargo-limit; fi
  if   command -v pacman       >/dev/null 2>&1 /dev/null; then sudo pacman -S --needed clang libfaketime libpulse i3-wm xorg-server-xephyr xorg-server-xvfb yarn; fi
  if   command -v apt-get      >/dev/null 2>&1 /dev/null; then sudo apt-get install -y build-essential clang i3-wm libfaketime libiw-dev libpulse-dev libx11-dev xserver-xephyr xvfb; fi

@check +CMDS:
    echo {{CMDS}} | xargs -n1 sh -c 'if ! command -v $1 >/dev/null 2>&1 /dev/null; then echo "$1 is required!"; exit 1; fi' bash

# build the crate
_build:
  cargo lbuild --quiet

# run `istat` in the terminal and interact with it
dev *args: _build
  cd ./scripts/run && RUST_BACKTRACE=1 RUST_LOG=istat=trace yarn start "$@"

# send an ipc event to the running debug version of istat (either `just dev` or `just debug`)
ipc *args: _build
  cargo lrun --quiet --bin istat-ipc -- --socket /tmp/istat-socket.dev "$@"

# run a binary
run bin *args:
  cargo lrun --bin istat-{{bin}} -- "$@"

# install locally, copy sample configuration and restart i3
install:
  cargo install --debug --offline --path .
  mkdir -p ~/.config/istat/
  -cp --no-clobber ./sample_config.toml ~/.config/istat/config.toml
  i3-msg restart

# start a nested X server with i3 and istat
debug dimensions="3800x200": _build
  Xephyr -ac -br -reset -terminate -screen {{dimensions}} :1 &
  until [ -e /tmp/.X11-unix/X1 ]; do sleep 0.1; done
  env -u I3SOCK DISPLAY=:1.0 i3-with-shmlog --config ./scripts/i3.conf

# test, test package and test AUR with package
test-publish:
  #!/usr/bin/env bash
  set -ex
  aur_target="./aur/target"
  rm -rf "$aur_target"

  cargo test
  cargo publish --dry-run --allow-dirty --target-dir "$aur_target"

  pushd aur
  source PKGBUILD
  cp "$(find . -name '*.crate')" "${source%%::*}"
  makepkg --cleanbuild --force --skipinteg --skipchecksums
  popd

# publish the create and update AUR package
publish: test-publish
  cargo publish
  just aur

# update the AUR package
aur:
  #!/usr/bin/env bash
  set -ex
  version=$(grep -m1 'version' ./Cargo.toml | cut -d' ' -f3)
  pushd aur
  sed --regexp-extended --in-place -E "0,/pkgver=.+$/{s/(pkgver=)(.+$)/\1${version}/}" ./PKGBUILD
  sed --regexp-extended --in-place -E "0,/sha512sums=.+$/{s/sha512sums=.+$/$(makepkg --geninteg)/}" ./PKGBUILD
  makepkg --printsrcinfo > .SRCINFO
  git commit --all --message $(echo $version | tr -d '"'})
  popd