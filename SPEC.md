# teLLeM spec v0.1 (2026-07-22)

An open-source Rust toolkit for AI-text forensics with receipts. Three verbs,
one engine, one philosophy: every finding cites the rule or signature that
fired, confidence is stated honestly, and below threshold it refuses to guess.

```
teLLeM lint <text> # find AI tells, cite each one
teLLeM fix <text> # rewrite the tells away (the de-AI pass)
teLLeM who <text> # attribute: which model family (and maybe version) wrote this
```

Public repo: github.com/ajd3v/teLLeM (MIT, private while pre-M1). The engine,
base rule pack, harvest/eval tooling, and the fingerprint catalog are public.
The catalog is the deliverable and the moat is the re-harvest loop (freshness),
not secrecy. What stays private: raw harvested corpora (API ToS exposure),
calibrated rule-pack weight overrides, and catalog refreshes ahead of the
public cycle ("secret sauce" = config, never code).

## Why this exists

Binary human-vs-AI detectors are oracles with unusable false-positive rates,
and every one of them hides its reasoning. teLLeM takes the opposite bet:
explainable, closed-set, evidence-cited findings. It never says "87% AI."
It says "12 tells fired: em-dash density 4.1/kwords (rule T001), 'delve' x2
(T014), uniform bullet openers 9/9 (S003)" or "closest known signature:
OpenAI GPT-5 family (see matched tells), margin below threshold: not calling
a version." Detection you can argue with, which is the only detection worth
publishing.

## Component 1, lint (the tell scanner)

Input: text (file, stdin, or library call). Output: findings with spans.

- Rule types:
  - **Lexical tells** (T-rules): tell-words and inflections (delve, showcase,
 leverage, meticulous, pivotal, testament, tapestry...), em/en dash usage,
 curly-vs-straight quote patterns, "not X, but Y" constructions.
  - **Phrase tells** (P-rules): cliches ("it's worth noting", "in today's
 fast-paced world", "shed light on", "at its core"...).
  - **Structural tells** (S-rules): uniform bullet openers, uniform sentence
 length (low variance), triadic-list overuse ("X, Y, and Z" density),
 paragraph-length uniformity, heading-case perfection, emoji-header
 patterns, bolded-lead-in bullet uniformity.
  - **Density scoring**: individual tells are weak, clustering is the signal.
 Score = weighted tells per kiloword, reported per rule with spans. Output
 bands: clean / seasoned / heavy, never a human-vs-AI verdict.
- Every rule has: id, name, description, examples, default weight, and a
 rationale string (shown in output: the receipt).
- Rule packs: TOML files. `packs/base.toml` ships public. Private packs load
 via `--pack` / config dir and can override weights or add rules.
- Format-aware: markdown/HTML/plaintext tokenizers so tells in prose aren't
 confused with code blocks or URLs.

## Component 2, fix (the rewriter)

- Deterministic rewrites keyed to rules: em-dash to comma/period restructure,
 tell-word to plain-word (case-preserving, inflection-aware), phrase deletion
 with punctuation cleanup, straight-quote normalization. This is a Rust
 superset of the existing TypeScript deAi util, the platform's `deai.ts`
 becomes a consumer via WASM/napi bindings once parity tests pass.
- Non-goal: LLM-based paraphrasing. fix is deterministic and reviewable, diff
 in/diff out. (An optional `--suggest` mode may LIST structural fixes, e.g.
 "vary these 9 identical bullet openers", but never rewrites structure
 silently.)
- Guarantee (CI-enforced, scoped to what fix touches): output re-linted never
 gains total tell weight, and every T/P finding that fix claims to handle is
 gone on re-lint. Total weight, not density, is the metric because deleting
 filler shrinks the word count and can raise density on tiny inputs.
 S-rules are out of scope by design and surface via `--suggest`.
 A provided test corpus round-trips with meaning intact (golden tests).

## Component 3, who (the model attributor)

The ambitious part. Closed-set model-family attribution from learned
fingerprints, honest about its limits.

### 3a. Harvester (`teLLeM harvest`)

- Config lists accessible models (OpenAI, Anthropic, Google, xAI, Meta,
 DeepSeek, Qwen, Mistral, local llama.cpp/Ollama, anything with an
 OpenAI-compatible endpoint).
- Prompt battery: N standardized prompts spanning registers (essay, email,
 resume bullets, technical explainer, story, listicle, code comments) x M
 samples x temperature grid. Battery is versioned so corpora are comparable
 across models and across time.
- Output: a corpus tree `corpora/<provider>/<model>/<battery-version>/*.txt`
 with full generation metadata (params, date, endpoint) per sample.
 Corpora from private keys stay private, the harvester itself is public so
 anyone can build their own.

### 3b. Tell miner (`teLLeM mine`)

- For each model corpus vs a reference baseline (human corpora: pre-2022
 books/wiki/news slices + the other models pooled):
  - token/lemma frequency deltas (log-odds with informative Dirichlet prior,
 the standard corpus-linguistics move),
  - distinctive n-grams and phrase templates,
  - structural stats (sentence-length distribution, punctuation profile,
 list/heading habits, em-dash rate, contraction rate),
  - burstiness/perplexity-free stylometrics only, no logits required, so it
 works on closed APIs.
- Output: a **fingerprint** per model: `catalog/<provider>/<model>.toml`,
 human-readable, diffable, versioned, each entry carrying its evidence
 (frequency ratios, sample counts). The catalog IS the deliverable: a
 browsable atlas of model tells that stays useful even when the classifier
 abstains.

### 3c. Attributor (`teLLeM who`)

- Scoring: log-likelihood ratio of the text under each fingerprint (naive
 Bayes over the mined features, exactly because naive Bayes is explainable:
 every feature's contribution is printable).
- Output contract (the thesis, enforced):
  - ranked candidates with per-feature receipts ("'moreover' rate matches
 Gemini 3 profile at 8x baseline..."),
  - calls a FAMILY only above a margin threshold, calls a VERSION only above
 a stricter one, otherwise: "insufficient signal, closest candidates: ...".
 Thresholds are not hand-picked: they are set from the eval harness to hit
 a held-out precision floor (family >= 95%, version >= 99%) and re-derived
 on every catalog change.
  - trained-set disclosure: "among the 14 models in this catalog" printed on
 every result. Out-of-catalog text gets "no catalog match", never a forced
 pick.
- Eval (`teLLeM eval`): held-out self-test per corpus, published confusion
 matrix in the README, re-run in CI on catalog changes. Accuracy claims in
 the README come from this harness or don't get made. Known hard cases
 documented: short texts, heavy human editing, RLHF convergence between
 vendors, model updates shifting fingerprints (catalog entries carry
 harvest dates, staleness is surfaced).

### Honest-limits section (ships in README verbatim)

Attribution is closed-set, stylometric, and probabilistic. It degrades on
short text, edited text, and unseen models. It is a forensic aid with cited
evidence, not proof of authorship, and it intentionally refuses more often
than it guesses.

## Architecture

- Workspace: `crates/tellem-core` (rules, scoring, fingerprints, no_std-clean
 where possible), `crates/tellem-cli` (clap), `crates/tellem-harvest`
 (async API clients), `crates/tellem-wasm` (wasm-bindgen), `crates/tellem-node`
 (napi-rs). Aho-Corasick for multi-pattern lexical/phrase scan, `fst` for
 large private packs, streaming scan, zero-copy spans.
- Performance target: lint at >1 GB/s single-thread on the base pack
 (criterion benches in CI, honest numbers in README). The point is that a
 full-manuscript scan is instant and embeddable anywhere, browser included.
- Bindings parity tests against the TS deai.ts corpus before the platform
 switches over.

## Consumers (day-one real users)

1. The job-search platform: deai pass via tellem-node, reviewer agent gains a
 "tell score" line item in the packet receipt.
2. The portfolio site: a live lint widget (WASM, client-side, zero API cost)
 next to the interrogation desk, and `who` behind it as a demo.
3. Owner materials: resumes/cover letters/essays linted pre-send.

## Milestones

- M1 (weekend): core + base pack + lint/fix CLI + benches + parity with
 deai.ts. Publishable.
- M2 (+few days): WASM/napi bindings, platform consumes it, site widget.
- M3 (+week): harvester + miner + catalog for 8-12 accessible models, `who`
 with eval harness and published confusion matrix.
- M4: community rule-pack contributions, catalog refresh automation (cron
 harvests, fingerprint drift tracking: "GPT-5.2 shifted em-dash rate 40%").

## Launch narrative (portfolio tie-in)

Essay: "I refuse to ship an AI detector", why oracle detectors are broken,
what evidence-cited linting + closed-set attribution can honestly do instead.
The repo demonstrates: Rust systems performance, corpus statistics, eval
discipline, and the refuse-below-threshold philosophy in a domain where every
competitor overclaims.

## Prior art and reusable assets (landscape survey, 2026-07-22, URLs verified)

Verdict from the survey: binary human-vs-AI detection is crowded and
discredited (OpenAI retired its own classifier at ~26% TPR), explainable
tell-linting is sparse (toy CLIs only), principled rule-based rewriting is
essentially unoccupied (the field is detector-evasion ware), and
**model-family attribution with interpretable fingerprints is genuinely open,
with zero Rust presence in any of the three capabilities.**

Validation that `who` works: locuslab's "Idiosyncrasies in Large Language
Models" (arXiv 2502.12150) hit 97.1% on 5-way ChatGPT/the assistant/Grok/Gemini/
DeepSeek attribution, with the signal rooted in word-level distributions that
survive paraphrase, exactly the feature class our miner extracts. LLMDet
(EMNLP 2023) validates the pre-recorded per-model n-gram dictionary approach
(classify without running the models), its stale 2023 model list proves why
the harvester/re-harvest loop is the moat.

Reusable inputs:
- **Rule seeds**: Wikipedia "Signs of AI writing" (CC BY-SA, the best curated
 tell catalog), awesome-slop JSON packs (CC0, incl. translationese),
 antislop-sampler's slop-phrase lists (Apache-2.0, 350 stars, ICLR 2026),
 slop-gate's ~40 tells. Per-rule receipts can cite the Science Advances
 15M-abstract study ("delve" 10.45x post-ChatGPT) for credibility.
- **Corpora/eval**: MAGE (27 LLMs, 7 families, 10 domains, best attribution
 training set), M4/M4GT-Bench (multilingual, EACL 2024), locuslab per-model
 response corpora, DetectRL (adversarial). slop-forensics (357 stars, MIT)
 is the closest methodological cousin: per-model over-represented word/
 bigram profiles + phylogenetic clustering, an offline Python toolkit with
 no classifier and no linting, we productize what it prototypes.
- **Design to emulate**: Vale's YAML style/rule format (our TOML packs should
 be near-isomorphic, consider a pack-to-Vale exporter for adoption), GLTR's
 token-level visual explanation UX.
- **Rust**: aho-corasick + fst (BurntSushi), HF tokenizers (real BPE token
 frequencies), linfa (explainable NB/logreg), criterion. The stylometry
 crate is dormant and basic, no Rust AI-text-detection/attribution crate
 exists.
- **Taxonomy note**: LLM-DetectAIve's 4-way (human / machine /
 machine-humanized / human-polished) matters, `lint` should eventually
 flag "humanized" texture too (the evasion arms race is detectable).
- Watermarking (SynthID etc.) requires generator cooperation, stylometric
 fingerprinting is the only third-party attribution path. Phantom dataset
 warning: "PANDORA" could not be verified to exist.
