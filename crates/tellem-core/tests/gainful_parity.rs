//! Parity gate for the gainful platform swap: every case in that repo's
//! deai.test.ts, run through base + the voice pack that carries its two
//! platform rules. Deviations are marked and are the whole point of the port.

use tellem_core::{Engine, Pack};

/// Ships in gainful as packages/shared/voice.toml. Inlined here so the public
/// repo keeps the platform's voice rules out of the public pack while still
/// proving the pack mechanism carries them.
const VOICE_PACK: &str = r#"
[meta]
name = "voice"
version = "0.1.0"

[[rules]]
id = "G001"
name = "roles-to-jobs"
kind = "word"
weight = 0.5
rationale = "job listings are jobs, roles is recruiter dialect (singular role is fine)"
[rules.swaps]
roles = "jobs"

[[rules]]
id = "G002"
name = "semicolon-clause-split"
kind = "regex"
pattern = '([A-Za-z0-9,)]); +([a-z])'
replace = '$1. $2'
weight = 1.0
rationale = "a semicolon joining two clauses is a model tell, two sentences read human"

[[rules]]
id = "G003"
name = "colon-clause-split"
kind = "regex"
pattern = '([A-Za-z0-9,)]): +([a-z])'
replace = '$1. $2'
weight = 1.0
rationale = "a colon dramatizing a lowercase continuation is a model tell, labels take a capital"
"#;

fn eng() -> Engine {
    Engine::from_packs(&[
        Pack::parse(tellem_core::BASE_PACK).unwrap(),
        Pack::parse(VOICE_PACK).unwrap(),
    ])
    .unwrap()
}

#[test]
fn resume_date_ranges_survive() {
    // The bug that started this project: "March 2021, March 2026" shipped to
    // production. A dash after a number is a range, and a range joins with "to".
    let e = eng();
    assert_eq!(
        e.fix("Data Manager | March 2021 – March 2026"),
        "Data Manager | March 2021 to March 2026"
    );
    assert_eq!(
        e.fix("Engineer | June 2026 – Present"),
        "Engineer | June 2026 to Present"
    );
    assert_eq!(e.fix("60 – 90 days"), "60 to 90 days");
    // DEVIATION from deai.ts, which left unspaced en-dashes alone.
    assert_eq!(e.fix("2020–2024"), "2020 to 2024");
}

#[test]
fn roles_become_jobs() {
    let e = eng();
    assert_eq!(
        e.fix("Target Roles: C++ and Python roles"),
        "Target Jobs: C++ and Python jobs"
    );
    // singular role is usually "the role of X", left alone
    assert_eq!(e.fix("the role of caching"), "the role of caching");
}

#[test]
fn dramatic_clause_split() {
    let e = eng();
    assert_eq!(
        e.fix("I trust them: they cite sources."),
        "I trust them. They cite sources."
    );
    assert_eq!(
        e.fix("I ship fast; the code is tested."),
        "I ship fast. The code is tested."
    );
    assert_eq!(
        e.fix("I run two: an app and a tool."),
        "I run two. An app and a tool."
    );
}

#[test]
fn legit_colons_spared() {
    let e = eng();
    for s in [
        "Skills: Python and C++",
        "meet at 12:30 today",
        "odds of 3:1 here",
        "see https://example.com now",
        "**Trust:** they cite their sources",
    ] {
        assert_eq!(e.fix(s), s, "rewrote a legit colon: {s}");
    }
}

#[test]
fn rest_of_the_ts_corpus() {
    let e = eng();
    for (input, want) in [
        (
            "I build software — and I ship it.",
            "I build software, and I ship it.",
        ),
        ("fast—reliable code", "fast, reliable code"),
        (
            "Architected a resilient framework",
            "Built a resilient framework",
        ),
        (
            "leveraging Python to showcase results",
            "using Python to show results",
        ),
        (
            "This underscores the meticulous work",
            "This highlights the careful work",
        ),
        (
            "a robust, comprehensive architecture that is scalable",
            "a robust, comprehensive architecture that is scalable",
        ),
        ("That being said, the system worked.", "The system worked."),
        ("It's worth noting that we shipped it.", "We shipped it."),
        ("This shed light on the issue.", "This shows the issue."),
        (
            "I fixed the timeout bug by chunking the API calls. It worked.",
            "I fixed the timeout bug by chunking the API calls. It worked.",
        ),
        ("", ""),
    ] {
        assert_eq!(e.fix(input), want, "on: {input}");
    }
}
