# tellem.gainful.work

Single page for teLLeM. Dependency free, one static server, everything in the
bench runs client side as wasm.

## Build

```sh
./scripts/build-wasm.sh                 # from the repo root, writes site/tellem/
./target/release/tellem mine corpora/with-rejection.jsonl -o site/tellem/catalog.toml
python3 site/atlas.py                   # pull the atlas row out of the catalog
python3 site/stamp.py                   # version the asset URLs
```

`stamp.py` matters. Static files are served immutable for a week, so an
unstamped rebuild reaches nobody. index.html is no-cache and carries the hashes.

## Run locally

```sh
cd site && PORT=8942 node serve.mjs
```

## Deploy

Needs a DNS record for tellem.gainful.work pointing at the box. Then:

```sh
TELLEM_HOST=user@box ./site/deploy/push.sh
```
