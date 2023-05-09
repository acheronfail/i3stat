setup:
  cd ./scripts/run && yarn

run:
  cargo lbuild
  cd ./scripts/run && yarn start

install:
  cargo install --path .
  mkdir -p ~/.config/staturs/
  cp ./sample_config.toml ~/.config/staturs/config.toml