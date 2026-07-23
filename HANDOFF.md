# teLLeM — session handoff (2026-07-22)

Read SPEC.md first (spec v0.1 + prior-art survey). This file is what a fresh
session needs to start building without re-deriving context.

## Current state

- SPEC.md is complete and owner-approved: three verbs (lint / fix / who) on
  one Rust engine, receipts on every finding, refuses below threshold.
- No code exists yet. M1 (SPEC "Milestones") is the next unit of work:
  cargo workspace, tellem-core + tellem-cli, base rule pack, lint + fix,
  criterion benches, parity tests against the platform's deai.ts.
- Name: **teLLeM** (display + repo, github.com/ajd3v/teLLeM, not yet created).
  Crates stay lowercase tellem-*. Local folder: ~/Projects/ajd3v/teLLeM (owner renamed from tell-em).
- License MIT. Public repo intent confirmed by owner. Secret sauce = private
  rule packs and fingerprint catalogs (config, never code).

## Owner voice pack: seed rules discovered 2026-07-22 (put in packs/owner.toml, PRIVATE)

These came from live owner corrections while editing his resume, they are both
his style guide and validated tell patterns:

1. Em/en dashes: never in prose (already in base deAi).
2. Semicolons: never ("I would never use semicolons"). Note: semicolons are
   also a top-cited AI tell, so a softer version belongs in the PUBLIC base
   pack as a density rule, not a ban.
3. "resume" never "résumé" (no accents).
4. Convince-words: qualifiers that defend a claim weaken it. Flagged twice on
   "paid" ("19 years of PAID experience", "the start of 19 years of
   continuous PAID software work"). Rule idea (public, general): flag
   defensive qualifiers near experience/quantity claims (paid, actual,
   real, genuine, proven, legitimate).
5. Identity vs verb claims: prefer "builds production AI systems" over "AI
   systems engineer" when unpaid/unconferred. Style guidance, probably a
   --suggest note, not a lint error.
6. "end to end" flagged as filler in summary context. Candidate for a
   weak-weight cliche rule.
7. Run-on tails: owner asked for a run-on split. S-rule candidate: sentence
   length outliers with 3+ comma-spliced clauses.
8. Deterministic rewrite hazard discovered in production: the deAi em-dash
   pass turned date ranges "2007 — 2024" into "2007, 2024" (nonsense) on the
   portfolio site. fix MUST special-case numeric ranges (en/em dash between
   years/numbers becomes "to", never a comma). Add a regression test for
   exactly this.
9. Domain-noun false positive discovered: "evaluation harness" got rewritten
   to "evaluation use" by the harness->use verb swap. fix needs POS-ish
   guards or a noun-context exception list. Regression test this too
   ("eval harness", "test harness", "wiring harness").

## Related assets and where they live

- The TS predecessor: ~/Projects/ajd3v/gainful/packages/shared/src/deai.ts
  (+ deai.test.ts, dewidow.ts). fix must reach parity with deAi() on its
  test corpus before the platform switches to tellem-node.
- Base-pack seed sources (verified URLs in SPEC prior-art section):
  Wikipedia "Signs of AI writing", awesome-slop JSON packs (CC0),
  antislop-sampler slop lists (Apache-2.0), slop-gate's ~40 tells.
- Attribution corpora for M3: MAGE, M4GT-Bench, locuslab llm-idiosyncrasies
  corpora. slop-forensics (sam-paech) is the methodological cousin to study.
- Harvest keys: the owner has working endpoints for DeepInfra (DeepSeek),
  Anthropic, and local llama.cpp/Ollama (see gainful .env and prod worker
  env LOCAL_OPENAI_*). Battery design in SPEC 3a.

## Integration targets (in priority order)

1. Platform deai pass via tellem-node (parity-gated).
2. Portfolio site lint widget (WASM, client-side) at ~/Projects/ajd3v/
   devaney-site, deployed as docker `ajd3v-site` on the gainful box
   (rsync + bash deploy/run-on-box.sh, see that repo's deploy/).
3. Reviewer agent "tell score" line in the platform's packet receipt.

## Owner style rules that apply to ALL user-facing text in this repo

README, CLI output, docs: no em-dashes, no semicolons in prose, no
convince-words, "resume" unaccented, brand phrase is "refuses to guess"
(NEVER "never confidently wrong"). The full standing rules live in the
memory file deai-default (auto-loaded in sessions for the gainful project;
sessions started in this folder should read this section instead).

## Suggested M1 order for the next session

1. `cargo new` workspace, tellem-core with Rule/Finding/Span types.
2. Base pack TOML schema + loader, seed ~60 rules from the sources above.
3. Aho-Corasick scan pass, markdown-aware segmentation, receipts output
   (human + JSON).
4. fix pass with the numeric-range and domain-noun guards from day one
   (regression tests above), then port deai.ts rules and run parity corpus.
5. criterion bench (target: honest numbers, publish whatever they are).
6. CI (GitHub Actions), then create the public repo and push.
