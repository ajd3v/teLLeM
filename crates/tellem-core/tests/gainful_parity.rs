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

[[rules]]
id = "G006"
name = "employer-then-technology"
kind = "regex"
pattern = '(?i)\b(?:at|with|for|while at) [A-Z][A-Za-z&.\- ]{2,40}[,;] [^.]{0,120}\b(?:TimescaleDB|PostgreSQL|Kafka|Terraform|Kubernetes|AWS|Azure|GCP|Snowflake|Databricks|React|Angular|Vue|Django|Rails|PHP|WordPress|Looker|Tableau|Spark|Airflow|dbt)\b'
weight = 0.4
rationale = "an employer named next to a technology is the role-conflation shape"

[[rules]]
id = "G007"
name = "unhedged-precise-figure"
kind = "regex"
pattern = '\$[0-9]+\.[0-9]{2,}|\b[0-9]{1,3},[0-9]{3}(?:,[0-9]{3})*\b|\b[0-9]{5,}\b'
weight = 0.3
rationale = "a precise figure reads as measured, confirm it is in the resume or a cited corpus item"
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

/// The two G rules added 2026-07-31 do not police style, they police SHAPES that
/// carried fabrications into real applications. Both are flag-only: neither can
/// be resolved without checking the resume and the career corpus, so the only
/// honest action is to make a human look.
mod grounding_shapes {
    use super::*;

    fn ids(text: &str) -> Vec<String> {
        eng()
            .lint(text)
            .findings
            .into_iter()
            .map(|f| f.rule_id)
            .collect()
    }

    #[test]
    fn flags_the_rivian_role_conflation() {
        // Real sentence from a sent cover letter. TimescaleDB and the backfiller
        // are real, but they belong to the 2013-2021 trading work, not to
        // Friendship Shelter. Every ingredient true, the attribution wrong.
        let t =
            "At Friendship Shelter, I used TimescaleDB and a custom C++ backfiller to ingest data.";
        assert!(ids(t).contains(&"G006".to_string()), "got {:?}", ids(t));
    }

    #[test]
    fn spares_a_sentence_that_names_only_the_employer() {
        let t = "At Friendship Shelter, I built the reporting platform program managers relied on.";
        assert!(!ids(t).contains(&"G006".to_string()));
    }

    #[test]
    fn flags_precise_figures_that_read_as_measured() {
        // "$0.022 per call on the first 20,000 calls" appears in no resume
        // version and in none of the 162 corpus items.
        let t = "That decision cost $0.022 per call on the first 20,000 calls.";
        assert!(ids(t).contains(&"G007".to_string()), "got {:?}", ids(t));
    }

    #[test]
    fn spares_a_year() {
        // "2024" is a year, not a claimed metric. The four-digit form flagged it.
        let t = "Tracked the HUD data-dictionary rename when the 2024 specification changed.";
        assert!(!ids(t).contains(&"G007".to_string()), "got {:?}", ids(t));
    }

    #[test]
    fn spares_round_counts_that_do_not_claim_precision() {
        let t = "The portal served roughly 40 staff across 16 programs.";
        assert!(!ids(t).contains(&"G007".to_string()), "got {:?}", ids(t));
    }

    #[test]
    fn neither_rule_rewrites_anything() {
        // Flag-only by design: a rewriter cannot know which job a technology
        // belongs to, or whether a number is real.
        let t = "At Acme Corp, I ran Kafka. Revenue was 1,250,000.";
        assert_eq!(eng().fix(t), t);
    }
}
