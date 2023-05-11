log := "RUST_LOG=staturs=trace"

setup:
  cd ./scripts/run && yarn

build:
  cargo lbuild

run: build
  cd ./scripts/run && {{log}} yarn start

install:
  cargo install --offline --debug --path .
  mkdir -p ~/.config/staturs/
  cp ./sample_config.toml ~/.config/staturs/config.toml

debug: install
  Xephyr -ac -br -reset -terminate -screen 3800x200 :1 &
  sleep 1
  DISPLAY=:1.0 i3-with-shmlog --config ./scripts/i3.conf
