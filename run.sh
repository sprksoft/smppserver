#!/bin/bash
# run.sh <extra_packages>

CARGO="cargo"

if [[ "$1" == "--fast" ]] ; then
  echo "using nightly..."
  CARGO="cargo +nightly"
  RUSTFLAGS="-Z threads=8"
fi

export ROCKET_CONFIG="smppgc/Rocket.toml"
$CARGO run --bin smppgc
