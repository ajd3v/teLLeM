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

# The site serves static files immutable for a week, so stamp the build hash
# into index.html (which is no-cache). widget.js reads it off its own URL and
# passes it to both imports, otherwise a rules update ships to nobody.
V="$(sha256sum "$SITE/tellem_wasm_bg.wasm" | cut -c1-8)"
sed -i -E "s|/tellem/widget\.js(\?v=[0-9a-f]+)?|/tellem/widget.js?v=$V|" "$SITE/../index.html"
echo "stamped build $V into index.html"

printf 'wasm: %s raw, %s gzipped\n' \
  "$(du -h "$SITE/tellem_wasm_bg.wasm" | cut -f1)" \
  "$(gzip -c "$SITE/tellem_wasm_bg.wasm" | wc -c | numfmt --to=iec)"
