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
    ('styles.css', r'href="/styles\.css(?:\?v=[^"\s]+)?"'),
    ('tellem/bench.js', r'src="/tellem/bench\.js(?:\?v=[^"\s]+)?"'),
]:
    # The loader passes its version to every lazy dependency.
    sources = [asset] if asset.endswith('.css') else [
        asset, 'tellem/tellem_wasm.js', 'tellem/tellem_wasm_bg.wasm', 'tellem/catalog.toml'
    ]
    digest = hashlib.sha256()
    for source in sources:
        digest.update((site / source).read_bytes())
    v = digest.hexdigest()[:8]
    attr = 'href' if asset.endswith('.css') else 'src'
    s = re.sub(pattern, f'{attr}="/{asset}?v={v}"', s)
    print(f'{asset} -> {v}')

html.write_text(s)
