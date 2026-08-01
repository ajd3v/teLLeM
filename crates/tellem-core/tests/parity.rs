//! Parity with the TS deAi() corpus, plus the two production regressions
//! (numeric dash ranges, domain-noun harness) where we deviate ON PURPOSE.

use tellem_core::{Engine, Pack};

fn eng() -> Engine {
    Engine::from_packs(&[Pack::parse(tellem_core::BASE_PACK).unwrap()]).unwrap()
}

#[test]
fn em_dash_to_comma() {
    let e = eng();
    assert_eq!(
        e.fix("I build software — and I ship it."),
        "I build software, and I ship it."
    );
    assert_eq!(e.fix("fast—reliable code"), "fast, reliable code");
}

#[test]
fn numeric_dash_ranges_become_to() {
    // DEVIATION from deai.ts (which produced "2007, 2024" in production).
    let e = eng();
    assert_eq!(e.fix("2020–2024"), "2020 to 2024");
    assert_eq!(e.fix("2007—2024"), "2007 to 2024");
    assert_eq!(
        e.fix("worked there 2020 – 2024 doing data"),
        "worked there 2020 to 2024 doing data"
    );
}

#[test]
fn word_swaps_preserve_case_and_inflection() {
    let e = eng();
    assert_eq!(
        e.fix("Architected a resilient framework"),
        "Built a resilient framework"
    );
    assert_eq!(
        e.fix("leveraging Python to showcase results"),
        "using Python to show results"
    );
    assert_eq!(
        e.fix("This underscores the meticulous work"),
        "This highlights the careful work"
    );
    assert_eq!(e.fix("UTILIZE IT"), "USE IT");
}

#[test]
fn domain_words_untouched() {
    let e = eng();
    let s = "a robust, comprehensive architecture that is scalable";
    assert_eq!(e.fix(s), s);
    assert_eq!(e.fix("the role of caching"), "the role of caching");
}

#[test]
fn harness_noun_guard() {
    // DEVIATION: deai.ts rewrote "evaluation harness" to "evaluation use" in prod.
    let e = eng();
    assert_eq!(
        e.fix("the eval harness runs nightly"),
        "the eval harness runs nightly"
    );
    assert_eq!(
        e.fix("a test harness for the parser"),
        "a test harness for the parser"
    );
    assert_eq!(e.fix("the wiring harness"), "the wiring harness");
    assert_eq!(e.fix("we harness Python daily"), "we use Python daily");
    assert_eq!(e.fix("They harnessed the data"), "They used the data");
}

#[test]
fn openers_deleted_and_recapitalized() {
    let e = eng();
    assert_eq!(
        e.fix("That being said, the system worked."),
        "The system worked."
    );
    assert_eq!(
        e.fix("It's worth noting that we shipped it."),
        "We shipped it."
    );
}

#[test]
fn cliche_phrases_rewritten() {
    let e = eng();
    assert_eq!(
        e.fix("This shed light on the issue."),
        "This shows the issue."
    );
    assert_eq!(
        e.fix("navigating the complex landscape of hiring"),
        "work through hiring"
    );
}

#[test]
fn clean_prose_untouched() {
    let e = eng();
    let s = "I fixed the timeout bug by chunking the API calls. It worked.";
    assert_eq!(e.fix(s), s);
    assert_eq!(e.fix(""), "");
}

#[test]
fn masked_regions_untouched() {
    let e = eng();
    let s = "Call `leverage()` here.";
    assert_eq!(e.fix(s), s);
    let md = "We delve deep.\n```\nlet x = delve — leverage;\n```\nMore delving.";
    let fixed = e.fix(md);
    assert!(
        fixed.contains("let x = delve — leverage;"),
        "code fence was rewritten: {fixed}"
    );
    assert!(fixed.starts_with("We look deep."));
    let lint = e.lint(md);
    assert!(lint.findings.iter().all(|f| !(15..46).contains(&f.start)));
}

#[test]
fn markdown_whitespace_survives() {
    // Regression c012aab: the TS pass ate nested-list indent. Hard line breaks
    // (two trailing spaces) are markdown too. Only mid-line runs collapse.
    let e = eng();
    let md = "- a\n    - nested delve item\n";
    assert_eq!(e.fix(md), "- a\n    - nested look item\n");
    assert_eq!(
        e.fix("a hard break  \nnext line"),
        "a hard break  \nnext line"
    );
    assert_eq!(e.fix("collapsed  here"), "collapsed here");
}

#[test]
fn pack_rule_ids_are_unique() {
    // Later packs override by id on purpose, so a dupe inside one pack is a
    // silent rule loss. 70 rules is past the count where eyeballing works.
    let pack = Pack::parse(tellem_core::BASE_PACK).unwrap();
    let mut seen = std::collections::HashSet::new();
    for r in &pack.rules {
        assert!(seen.insert(r.id.clone()), "duplicate rule id {}", r.id);
    }
}

#[test]
fn curly_quotes_normalized() {
    let e = eng();
    assert_eq!(e.fix("“fine” work, it’s good"), "\"fine\" work, it's good");
}

#[test]
fn lint_reports_receipts() {
    let e = eng();
    let r = e.lint("We delve into a rich tapestry — a testament to synergy.");
    let ids: Vec<&str> = r.findings.iter().map(|f| f.rule_id.as_str()).collect();
    // P043 not T024: the phrase rule wins the leftmost-longest match on
    // "a testament to", which is what keeps fix from writing "a sign to".
    for id in ["T013", "T033", "T001", "P043", "T036"] {
        assert!(ids.contains(&id), "missing {id} in {ids:?}");
    }
    assert!(r.findings.iter().all(|f| !f.rationale.is_empty()));
    assert_eq!(r.band, tellem_core::Band::Heavy);
}

#[test]
fn structural_bullets_flagged() {
    let e = eng();
    let doc = "Intro line.\n\n- Built the API\n- Built the UI\n- Built the CI\n- Built the docs\n- Built the tests\n";
    let r = e.lint(doc);
    assert!(
        r.findings.iter().any(|f| f.rule_id == "S001"),
        "{:?}",
        r.findings
    );
}

/// The guarantee from SPEC component 2, CI-enforced on this golden corpus:
/// total tell weight never increases, and every finding from a rule fix
/// claims to handle is gone on re-lint.
#[test]
fn fix_monotonic_and_complete() {
    let e = eng();
    let corpus = [
        "It's worth noting that we delve into the ever-evolving landscape of \
         software. Our meticulous approach leverages cutting-edge tools — a \
         testament to synergy. Needless to say, the possibilities are endless.",
        "I hope this email finds you well. In today's fast-paced world, we \
         utilize a myriad of frameworks to showcase our multifaceted, \
         transformative platform. Moreover, it boasts seamless integration.",
        "The parser handles UTF-8. The tests pass on CI. Nothing fancy here, \
         and that is the point of the whole exercise.",
    ];
    for text in corpus {
        let before = e.lint(text);
        let fixed = e.fix(text);
        let after = e.lint(&fixed);
        assert!(
            after.total_weight <= before.total_weight + f32::EPSILON,
            "weight rose {} -> {} on: {text}\nfixed: {fixed}",
            before.total_weight,
            after.total_weight
        );
        let leftover: Vec<_> = after.findings.iter().filter(|f| f.fixable).collect();
        assert!(
            leftover.is_empty(),
            "fixable findings survived fix: {leftover:?}\nfixed: {fixed}"
        );
    }
}

#[test]
fn phrase_rules_beat_the_bare_word_swap() {
    // The site widget made these visible: "a myriad of X" swapped word-by-word
    // reads "a many of X", and "a testament to X" reads "a sign to X".
    let e = eng();
    assert_eq!(
        e.fix("It showcases a myriad of possibilities, a testament to synergy."),
        "It shows many possibilities, a sign of synergy."
    );
    assert_eq!(e.fix("Myriads of tools"), "Many tools");
    // the bare word still swaps where no construction applies
    assert_eq!(e.fix("a myriad landscape"), "a many landscape");
}

#[test]
fn html_markup_is_not_prose() {
    // The site is the HTML consumer the mask.rs note was waiting for. Tags and
    // attributes are markup, script and style bodies are code, and <code> is
    // masked for the same reason backticks are: naming a rule is a mention.
    let e = eng();
    let page = r#"<p class="lede delve showcase">We look at it.</p>
<script>const leverage = "delve";</script>
<style>.x { content: "utilize"; }</style>
<p>The word <code>delve</code> is a rule, not a habit.</p>"#;
    let r = e.lint(page);
    assert_eq!(
        r.findings.len(),
        0,
        "markup read as prose: {:?}",
        r.findings
    );

    // Prose in the same file still gets read.
    let mixed = "<p>We delve into the tapestry.</p>";
    assert!(
        e.lint(mixed).findings.iter().any(|f| f.rule_id == "T013"),
        "prose inside tags was skipped"
    );
    // and fix leaves the markup alone
    assert_eq!(
        e.fix(r#"<a href="/x" class="leverage">We leverage it.</a>"#),
        r#"<a href="/x" class="leverage">We use it.</a>"#
    );
}

#[test]
fn every_rule_can_actually_fire() {
    // A word or phrase rule with neither `patterns` nor `swaps` compiles fine,
    // loads fine, and matches nothing forever. Two flag-only rules shipped that
    // way: the author dropped the swaps to stop the rewrite, which also removed
    // the only thing the matcher had to look for. Nothing else notices, because
    // a rule that never fires produces no output to be wrong.
    let pack = Pack::parse(tellem_core::BASE_PACK).unwrap();
    let dead: Vec<&str> = pack
        .rules
        .iter()
        .filter(|r| match r.kind {
            tellem_core::Kind::Word => r.patterns.is_empty() && r.swaps.is_empty(),
            tellem_core::Kind::Phrase => r.patterns.is_empty(),
            tellem_core::Kind::Regex => r.pattern.is_none(),
        })
        .map(|r| r.id.as_str())
        .collect();
    assert!(dead.is_empty(), "rules that can never match: {dead:?}");
}

#[test]
fn the_base_pack_stays_domain_agnostic() {
    // The pack is for generated prose anywhere. Rules arrived once framed
    // around one field, because that is the corpus that happened to be in front
    // of whoever added them, and rationales are published output: a
    // resume-shaped rationale ships a resume-shaped tool. The words themselves
    // were fine, the justifications were not.
    //
    // A rule that only makes sense in one field belongs in a pack for that
    // field. G006 and G007 in the platform's voice pack are the example: they
    // flag employer-and-technology and unhedged figures, which mean nothing
    // outside an application, and they correctly live there rather than here.
    const FIELDS: [&str; 8] = [
        "resume",
        "cover letter",
        "cover-letter",
        "candidate",
        "hiring",
        "recruiter",
        "applicant",
        "job application",
    ];
    let pack = Pack::parse(tellem_core::BASE_PACK).unwrap();
    let leaked: Vec<(&str, &str)> = pack
        .rules
        .iter()
        .filter_map(|r| {
            let low = r.rationale.to_lowercase();
            FIELDS
                .iter()
                .find(|f| low.contains(*f))
                .map(|f| (r.id.as_str(), *f))
        })
        .collect();
    assert!(
        leaked.is_empty(),
        "base pack rationales name a single field: {leaked:?}"
    );
}
