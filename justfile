setup:
  cd ./scripts/run && yarn

run:
  cargo lbuild
  cd ./scripts/run && yarn start
