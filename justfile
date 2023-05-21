log := "RUST_LOG=istat=trace"

default:
  just --list

setup:
  cd ./scripts/run && yarn

build:
  cargo lbuild --quiet

dev *args: build
  cd ./scripts/run && {{log}} yarn start {{args}}

ipc *args: build
  cargo lrun --quiet --bin istat-ipc -- --socket /tmp/istat-socket.dev {{args}}

install:
  cargo install --offline --debug --path .
  mkdir -p ~/.config/istat/
  cp ./sample_config.toml ~/.config/istat/config.toml

debug: install
  Xephyr -ac -br -reset -terminate -screen 3800x200 :1 &
  sleep 1
  env -u I3SOCK DISPLAY=:1.0 i3-with-shmlog --config ./scripts/i3.conf

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