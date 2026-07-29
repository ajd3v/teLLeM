# teLLeM session handoff (2026-07-22)

Read SPEC.md first (spec v0.1 + prior-art survey). This file is what a fresh
session needs to start building without re-deriving context.

## Current state

- SPEC.md is complete and owner-approved: three verbs (lint / fix / who) on
 one Rust engine, receipts on every finding, refuses below threshold.
- M1 SHIPPED (2026-07-22): workspace, tellem-core + tellem-cli, 70-rule base
 pack, lint + fix, 13 parity/regression tests, criterion bench (honest
 number: ~93 MiB/s single thread, README documents the gap to the 1 GB/s
 target), GitHub Actions CI green.
- M2 bindings SHIPPED: tellem-wasm (lint_json + fix, wasm32 target checked
 in CI) and tellem-node (napi, deAi drop-in + lintJson + setPack, node parity
 test in CI).
- Integration 1 SHIPPED (2026-07-28): gainful's deAi() calls tellem-node. The
 addon is VENDORED at gainful packages/tellem/tellem.node (prebuilt, refreshed
 by its update.sh), not built in CI. Its glibc floor is 2.34 vs 2.39 in the
 playwright image, so the Dockerfile needed no rust. Two bundling constraints
 that will bite again: tellem.ts and email.ts had to leave the
 @gainful/shared barrel (client components import it), and @gainful/tellem
 needs an explicit config.externals entry because serverExternalPackages does
 not externalize workspace packages. The platform's own rules live in
 gainful packages/tellem/voice.toml, mirrored by tests/gainful_parity.rs here.
 Integration 3 SHIPPED (2026-07-28): the reviewer agent gets a deterministic
 tell scan before it judges tone, and PacketTells lands on cv_versions.review
 for the apply-room receipt. All three integration targets are done.
- Integration 2 SHIPPED (2026-07-28): the "Tells" bench on devaney-site, wasm
 running client-side. scripts/build-wasm.sh here builds it, vendors it into
 that repo, and stamps the build hash into index.html. That stamp matters:
 the site serves every non-HTML file immutable for a week, so without a
 versioned URL a rules change reaches nobody for seven days.
- Putting the engine's output on a page is the best rule-quality test found so
 far. It immediately showed two word swaps producing broken English ("a many
 of possibilities", "a sign to synergy"), now fixed by P042 and P043. Run the
 bench against real prose before trusting a new swap rule.
- M3 PARTLY SHIPPED (2026-07-28): mine / who / eval work and clear the
 pre-registered gate. 95.0% precision at 84.3% coverage, 88.5% forced choice,
 3.5% false positives on 32,523 out-of-catalog texts. Corpus is
 locuslab/llm-idiosyncrasies (MIT), five commercial families from LATE 2024,
 plus MAGE human text as the rejection class. Naive Bayes was the spec's
 choice and could not reach the floor (72.7%), logistic regression can and is
 still a dot product, so receipts still print.
 The harvester IS written and tested against a mock, see harvest.toml.example.
 It cannot run until the gateway is re-authed, see below. Once it can:
   tellem harvest harvest.toml
 is safe to interrupt and safe to re-run, so put it on a cron slice.
- site/ is tellem.gainful.work, watercolour, all three verbs client side.
 Needs a DNS record before deploy/push.sh will do anything.
- GATEWAY, corrected 2026-07-28 after actually retesting. FOUR of six families
 work right now and harvesting can start today: claude-4 (ag/claude-sonnet-4-6),
 gemini-3 (ag/gemini-3-flash), gpt-oss (ag/gpt-oss-120b-medium), and
 deepseek-flash (OpenCodeDeepSeek, free tier, returns empty bodies often).
 Only TWO are genuinely blocked on credentials: cx/gpt-5.5 gives 401 token
 expired, xai/grok-4 gives 403 OAuth2 bad credentials.
 The earlier "gateway is down across the board" note was wrong twice over.
 ag/* backends stream by default, so a probe that parses plain JSON reads a
 working model as an empty body: send stream:false. And gc/gemini-3-pro-preview
 404s because that model id does not exist, not because of auth. Five other
 gemini aliases answer fine.
 Prefer model ids whose upstream reports a specific version. ag/gemini-3.5-flash-low
 answers as "gemini-default", which cannot be audited later.
- Name: **teLLeM** (display + repo, github.com/ajd3v/teLLeM, created 2026-07-22
 as PRIVATE, flip to public at M1 publishable).
 Crates stay lowercase tellem-*. Local folder: ~/Projects/ajd3v/teLLeM (owner renamed from tell-em).
- License MIT. Public repo intent confirmed by owner.
- Spec grill session (2026-07-22) settled three decisions, now in SPEC.md:
  1. fix guarantee is scoped: monotonic (never scores worse) plus complete
 for claimed T/P rules. S-rules out of scope, surfaced via --suggest.
  2. The 1 GB/s lint target STAYS (owner call, aspirational constraint).
  3. Fingerprint catalog is PUBLIC (it is the deliverable, moat = freshness).
 Private: raw corpora, calibrated weight overrides, pre-release refreshes.
 Bonus: who thresholds derive from the eval harness precision floor
 (family >= 95%, version >= 99%), never hand-picked.

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
 pass turned date ranges "2007, 2024" into "2007, 2024" (nonsense) on the
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

Note for linting sessions: quoted strings in this file and in SPEC.md name
banned forms as rule definitions or historical examples. They are mentions,
not usage. Both docs are otherwise compliant as of 2026-07-22.

README, CLI output, docs: no em-dashes, no semicolons in prose, no
convince-words, "resume" unaccented, brand phrase is "refuses to guess"
(NEVER "never confidently wrong"). The full standing rules live in the
memory file deai-default, auto-loaded in sessions for the gainful project.
Sessions started in this folder should read this section instead.

## Suggested M1 order for the next session

1. `cargo new` workspace, tellem-core with Rule/Finding/Span types.
2. Base pack TOML schema + loader, seed ~60 rules from the sources above.
3. Aho-Corasick scan pass, markdown-aware segmentation, receipts output
 (human + JSON).
4. fix pass with the numeric-range and domain-noun guards from day one
 (regression tests above), then port deai.ts rules and run parity corpus.
5. criterion bench (target: honest numbers, publish whatever they are).
6. CI (GitHub Actions), then create the public repo and push.
