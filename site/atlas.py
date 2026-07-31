#!/usr/bin/env python3
"""Pull one catalog row into a small JSON the page can fetch.

The site used to hardcode these rates. They are catalog output, so hardcoding
them meant a remine would silently leave the page stating numbers the catalog
no longer holds. Same rule the README lives by: a number comes from the tool
or it does not get written.

Usage: python3 site/atlas.py [feature]   (default w:delve)
"""
import json
import pathlib
import re
import sys

feature = sys.argv[1] if len(sys.argv) > 1 else 'w:honest'
site = pathlib.Path(__file__).parent
catalog = site / 'tellem' / 'catalog.toml'
text = catalog.read_text()

families = json.loads(re.search(r'^families = (\[.*?\])$', text, re.M).group(1))
block = re.search(
    rf'^\[features\."{re.escape(feature)}"\]$(.*?)(?=^\[|\Z)', text, re.M | re.S)
if not block:
    raise SystemExit(f'{feature} is not in {catalog.name}')
rates = json.loads(re.search(r'^rate = (\[.*?\])$', block.group(1), re.M).group(1))

rows = sorted(zip(families, rates), key=lambda r: -r[1])
out = site / 'tellem' / 'atlas.json'
out.write_text(json.dumps({'feature': feature, 'rows': rows}))
print(f'{feature}: ' + ', '.join(f'{f} {r:.4f}' for f, r in rows))
print(f'-> {out.relative_to(site)} ({out.stat().st_size} bytes)')
