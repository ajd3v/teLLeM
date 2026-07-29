#!/usr/bin/env bash
# Build the browser bundle and drop it into BOTH consumers: this repo's own
# site/, and the bench section of the portfolio site. Run after changing rules.
# Both sites serve static files immutable for a week and both stamp the build
# hash into their (no-cache) index.html, so an unstamped copy would ship to
# nobody for seven days.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
PORTFOLIO="${DEVANEY_SITE:-$HOME/Projects/ajd3v/devaney-site}"

npx --yes wasm-pack@latest build "$HERE/crates/tellem-wasm" \
  --target web --release --out-dir "$HERE/target/wasm-site"

copy_to() {
  cp "$HERE/target/wasm-site/tellem_wasm.js" \
     "$HERE/target/wasm-site/tellem_wasm_bg.wasm" "$1/tellem/"
}

# 1. tellem.gainful.work. stamp.py versions styles.css and bench.js together.
copy_to "$HERE/site"
python3 "$HERE/site/stamp.py"

# 2. The portfolio bench, if it is checked out. Its widget.js is versioned by
#    the engine it loads, same idea, different file name.
if [ -d "$PORTFOLIO/tellem" ]; then
  copy_to "$PORTFOLIO"
  V="$(sha256sum "$PORTFOLIO/tellem/tellem_wasm_bg.wasm" | cut -c1-8)"
  sed -i -E "s|/tellem/widget\.js(\?v=[0-9a-f]+)?|/tellem/widget.js?v=$V|" "$PORTFOLIO/index.html"
  echo "portfolio stamped $V (commit that repo too)"
else
  echo "portfolio site not found at $PORTFOLIO, skipped"
fi

printf 'wasm: %s raw, %s gzipped\n' \
  "$(du -h "$HERE/site/tellem/tellem_wasm_bg.wasm" | cut -f1)" \
  "$(gzip -c "$HERE/site/tellem/tellem_wasm_bg.wasm" | wc -c | numfmt --to=iec)"
