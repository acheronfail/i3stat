_default:
  just --list

# setup repository and install dev dependencies
setup:
  cd ./scripts/run && yarn

# build the crate
_build:
  cargo lbuild --quiet

# run `istat` in the terminal and interact with it
dev *args: _build
  cd ./scripts/run && RUST_LOG=istat=trace yarn start {{args}}

# send an ipc event to the running debug version of istat (either `just dev` or `just debug`)
ipc *args: _build
  cargo lrun --quiet --bin istat-ipc -- --socket /tmp/istat-socket.dev {{args}}

# install locally, copy sample configuration and restart i3
install:
  cargo install --offline --path .
  mkdir -p ~/.config/istat/
  cp --no-clobber ./sample_config.toml ~/.config/istat/config.toml || true
  i3-msg restart

# start a nested X server with i3 and istat
debug: install
  Xephyr -ac -br -reset -terminate -screen 3800x200 :1 &
  sleep 1
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