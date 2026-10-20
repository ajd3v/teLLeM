#!/usr/bin/env bash
# Push the site to the box and restart it. Run from the teLLeM repo root.
# The wasm and catalog are build artifacts, so build them BEFORE this:
#   ./scripts/build-wasm.sh && ./target/release/tellem mine <corpus> -o site/tellem/catalog.toml
#   python3 site/stamp.py
set -euo pipefail
HOST="${TELLEM_HOST:-deploy@108.61.220.243}"
SSH=(ssh -i "${TELLEM_KEY:-$HOME/.ssh/gainful_deploy}" -o BatchMode=yes)
node site/serve.test.mjs
rsync -az --delete --exclude node_modules -e "${SSH[*]}" ./site/ "$HOST:/home/deploy/tellem-site/"
"${SSH[@]}" "$HOST" 'cd /home/deploy/tellem-site && ./deploy/run-on-box.sh'
