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

### A 2026 catalog

Four families harvested through the local gateway at about 300 samples each,
plus the rejection class. Held out by prompt, threshold derived by `eval`:

| | |
|---|---|
| precision at the floor | 95.1% |
| coverage there | 91.0% |
| false positives, out of catalog | 1.9% |

It clears the floor on 300 samples per family where the 2024 catalog used
10,340, which is worth knowing before anyone plans a long harvest.

### Have the models converged?

The assumption going in was yes, and that a 2026 catalog would be harder. The
data says the question is too coarse to have one answer.

Both eras cut to the same shape, four families at 295 samples plus an equal
rejection class, and evaluated identically:

| word cap | 2024 | 2026 |
|---|---|---|
| none | 95.5% at 46.4% coverage | 95.1% at 91.4% coverage |
| 120 | 95.7% at 40.8% coverage | 95.7% at 92.5% coverage |
| 50 | 96.2% at 14.3% coverage | 96.3% at 73.3% coverage |

2026 is far MORE separable, and it survives capping length, so it is not just
that the newer models differ in how much they say. Restricting to the three
vendors present in both eras keeps the direction, 84.4% against 89.4% uncapped.

Pairwise is where it gets specific. Two-class, 280 samples each, 50-word cap:

| | 2024 | 2026 |
|---|---|---|
| Anthropic against Google | 88.5% | 85.9% |
| Anthropic against OpenAI | 90.1% | 89.3% |
| Google against OpenAI | 73.3% | 95.2% |

Anthropic and Google drifted slightly together. Anthropic and OpenAI held
station. Google and OpenAI, the hardest pair to tell apart in 2024, are now the
easiest by a wide margin. "Models are converging" is not what happened, and an
earlier version of this section said 2026 looked slightly harder because the
two-class test it ran happened to land on the one pair that did converge.

Caveat worth carrying: gemini-3-flash is a faster tier than gemini-1.5-pro, so
that row compares vendors rather than equivalent tiers.

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
