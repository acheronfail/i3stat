log := "RUST_LOG=rstat=trace"

default:
  just --list

setup:
  cd ./scripts/run && yarn

build:
  cargo lbuild --quiet

dev *args: build
  cd ./scripts/run && {{log}} yarn start {{args}}

ipc *args: build
  cargo lrun --quiet --bin rstat-ipc -- --socket /tmp/rstat-socket.dev {{args}}

install:
  cargo install --offline --debug --path .
  mkdir -p ~/.config/rstat/
  cp ./sample_config.toml ~/.config/rstat/config.toml

debug: install
  Xephyr -ac -br -reset -terminate -screen 3800x200 :1 &
  sleep 1
  env -u I3SOCK DISPLAY=:1.0 i3-with-shmlog --config ./scripts/i3.conf
