#!/bin/bash
# run.sh <extra_packages>

ROOT="$(dirname $0)"

GIT_HOOK=".git/hooks/pre-commit"
# set git hook

if ! ls "$ROOT/$GIT_HOOK"  ; then
  echo "Generating git hook..."
  echo "#!/bin/bash
# A hook to regenerate v1.js on pre-commit
smppgc/gen_js.sh --git-add" > "$ROOT/$GIT_HOOK" || exit 1
  sudo chmod +x "$ROOT/$GIT_HOOK" || exit 1
fi

smppgc/gen_js.sh || exit 1

CARGO="cargo"

if [[ "$1" == "--fast" ]] ; then
  echo "using nightly..."
  CARGO="cargo +nightly"
  RUSTFLAGS="-Z threads=8"
fi

export ROCKET_CONFIG="smppgc/Rocket.toml"
$CARGO run --bin smppgc
