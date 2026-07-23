# teLLeM

AI-text forensics with receipts. Every finding cites the rule that fired,
confidence is stated honestly, and below threshold it refuses to guess.

```
tellem lint <file>   # find AI tells, cite each one
tellem fix  <file>   # rewrite the tells away, deterministic and reviewable
tellem who  <file>   # closed-set model attribution (coming in M3)
```

teLLeM never says "87% AI". It says:

```
T013 delve       26..31   "delve"
     delve rose 10x in abstracts post ChatGPT (Science Advances, 15M abstracts)
T001 em-dash     53..56   "—"
     em-dash density is the top measurable tell, humans reach for commas
7 findings, 4.5 tells/kword over 1550 words, band: seasoned
```

Bands are clean, seasoned, heavy. There is no human-vs-AI verdict, because
individual tells are weak and clustering is the only real signal. Detection
you can argue with is the only detection worth publishing.

## lint

Rules live in TOML packs (`packs/base.toml` ships with the binary, extra
packs load with `--pack` and can override by rule id). Lexical tells (T),
phrase tells (P), and structural tells (S, computed in code: uniform bullet
openers, uniform sentence length, triadic overuse). Markdown code fences,
inline code, and URLs are masked, tells in code are not tells.

## fix

Deterministic rewrites keyed to rules: dashes to commas, numeric ranges to
"to" (never a comma), tell-words to plain words with case and inflection
preserved, filler phrases deleted with punctuation cleanup. No LLM anywhere.

CI-enforced guarantee: fixed output re-linted never gains total tell weight,
and every finding from a rule fix claims to handle is gone. Structural tells
are out of scope by design and will surface via `--suggest`.

## Benchmarks (honest numbers)

criterion, single thread, base pack, 1.2 MB of prose:

| bench | throughput |
|---|---|
| lint | ~93 MiB/s |

A full manuscript lints in about 11 ms. The 1 GB/s target from the spec is
not met yet, the gap is real and documented, and numbers in this table come
from `cargo bench` or they do not get written.

## Honest limits

Attribution is closed-set, stylometric, and probabilistic. It degrades on
short text, edited text, and unseen models. It is a forensic aid with cited
evidence, not proof of authorship, and it intentionally refuses more often
than it guesses.

## License

MIT
