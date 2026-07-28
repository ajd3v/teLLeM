#!/usr/bin/env bash
# Build the browser bundle and drop it in the portfolio site. Run after changing
# rules, then commit the new tellem_wasm_bg.wasm there.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
SITE="${DEVANEY_SITE:-$HOME/Projects/ajd3v/devaney-site}/tellem"

npx --yes wasm-pack@latest build "$HERE/crates/tellem-wasm" \
  --target web --release --out-dir "$HERE/target/wasm-site"

cp "$HERE/target/wasm-site/tellem_wasm.js" \
   "$HERE/target/wasm-site/tellem_wasm_bg.wasm" "$SITE/"

printf 'wasm: %s raw, %s gzipped\n' \
  "$(du -h "$SITE/tellem_wasm_bg.wasm" | cut -f1)" \
  "$(gzip -c "$SITE/tellem_wasm_bg.wasm" | wc -c | numfmt --to=iec)"
