#!/usr/bin/env python3
"""Stamp asset URLs in index.html with a content hash.

Static files are served immutable for a week, so without a versioned URL a
rebuild reaches nobody. index.html is no-cache and carries the stamps.
Run after ../scripts/build-wasm.sh or any styles.css edit."""
import hashlib, pathlib, re

site = pathlib.Path(__file__).parent
html = site / 'index.html'
s = html.read_text()

for asset, pattern in [
    ('styles.css', r'href="/styles\.css(?:\?v=[0-9a-f]+)?"'),
    ('tellem/bench.js', r'src="/tellem/bench\.js(?:\?v=[0-9a-f]+)?"'),
]:
    # bench.js is versioned by the engine it loads, not by its own bytes
    src = site / ('tellem/tellem_wasm_bg.wasm' if asset.endswith('bench.js') else asset)
    v = hashlib.sha256(src.read_bytes()).hexdigest()[:8]
    attr = 'href' if asset.endswith('.css') else 'src'
    s = re.sub(pattern, f'{attr}="/{asset}?v={v}"', s)
    print(f'{asset} -> {v}')

html.write_text(s)
