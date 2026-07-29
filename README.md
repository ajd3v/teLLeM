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
packs load with `--pack` and can override by rule id). `--fail-on seasoned`
exits 1 so lint can gate CI. Lexical tells (T),
phrase tells (P), and structural tells (S, computed in code: uniform bullet
openers, uniform sentence length, triadic overuse). Markdown code fences,
inline code, and URLs are masked, tells in code are not tells.

## fix

Deterministic rewrites keyed to rules: dashes to commas, numeric ranges to
"to" (never a comma), tell-words to plain words with case and inflection
preserved, filler phrases deleted with punctuation cleanup. No LLM anywhere.
Markdown whitespace that carries meaning (list indent, hard line breaks)
survives the cleanup pass. `fix -w` rewrites in place, because `fix f > f`
truncates the file.

CI-enforced guarantee: fixed output re-linted never gains total tell weight,
and every finding from a rule fix claims to handle is gone. Structural tells
are out of scope by design and will surface via `--suggest`.

## who

Closed-set attribution over a mined catalog. `mine` learns a fingerprint per
model family from a labelled corpus, `who` scores text against it, and `eval`
derives the confidence threshold from a held-out split rather than picking one.

Held out on 10,190 samples (five families, split by prompt so no prompt appears
on both sides), corpus is locuslab/llm-idiosyncrasies, late-2024 models:

| | |
|---|---|
| forced choice, no abstention | 88.5% |
| at the 95% precision floor | 95.0% precision, 84.3% coverage |
| false positives on 32,523 out-of-catalog texts | 3.5% |

The threshold is derived, never hand-picked. Precision is the invariant and
coverage is whatever falls out of it, so `who` refuses on about one text in six
rather than guessing. Every call prints the features that decided it, with the
family rate against the corpus baseline.

The catalog carries a rejection class, and a text that ranks into it gets no
catalog match at any confidence. This is load-bearing rather than decorative: a
five-way softmax sums to one, so without a rejection option the classifier must
name one of five whatever it is shown, and it named a model for 45% of genuine
human writing. With the rejection class that falls to 0.9% on human text and
3.5% across a control that is mostly 27 uncatalogued models.

Naive Bayes was tried first and reached 72.7% forced choice, clearing the floor
only at 34% coverage. Logistic regression on the same features is still a dot
product, so every contribution still prints. Explainability was the constraint,
not the specific classifier.

### Does a fingerprint survive a model generation?

The catalog is built from late-2024 models, and everything frontier since has
been tuned harder toward the same register, so the obvious worry is that the
signal has flattened. `scripts/era-compare.py` holds everything fixed except
the era: same two vendors, same prompt ids, same sample count, same two-class
task.

| | forced choice | at the 95% floor |
|---|---|---|
| 2024, claude-3-5-sonnet against gemini-1.5-pro | 93.8% | 96.9% coverage |
| 2026, claude-sonnet-4-6 against gemini-3-flash | 90.6% | 82.8% coverage |

The fingerprints survive. Across four holdout splits the 2026 pair came out
about four points harder in three of them, which points the way you would
expect, but roughly 130 held-out samples per era puts the 95% interval near
five points. The gap is suggestive and it is not significant. Read it as
"still works, watch this number as the catalog refreshes", not as a measured
rate of decay.

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
