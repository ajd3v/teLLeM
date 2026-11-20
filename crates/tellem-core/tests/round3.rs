//! Regressions from the supplied PR #13, PR #14 and Wikipedia excerpts.
use std::sync::OnceLock;
use tellem_core::{Engine, Pack};

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        Engine::from_packs(&[Pack::parse(tellem_core::BASE_PACK).unwrap()]).unwrap()
    })
}

fn finds(text: &str, id: &str) {
    let e = engine();
    let report = e.lint(text);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == id)
        .unwrap_or_else(|| panic!("missing {id} on {text:?}: {:?}", report.findings));
    assert!(!f.fixable);
    assert!(!f.rationale.is_empty());
    assert_eq!(f.rationale.lines().count(), 1);
    assert!(f.start < f.end && text.get(f.start..f.end).is_some());
    let fixed = e.fix(text);
    let after = e.lint(&fixed);
    assert!(
        after.total_weight <= report.total_weight + f32::EPSILON,
        "weight rose on {text:?}"
    );
    assert!(
        after.findings.iter().all(|f| !f.fixable),
        "fixable finding survived on {text:?}"
    );
    for masked in [
        format!("```text\n{text}\n```"),
        format!("<code>{text}</code>"),
    ] {
        assert!(!e.lint(&masked).findings.iter().any(|f| f.rule_id == id));
    }
}

fn no_pairing(text: &str) {
    let report = engine().lint(text);
    assert!(
        report.findings.is_empty(),
        "finding on clean prose {text:?}: {:?}",
        report.findings
    );
}

#[test]
fn relabel_pairing_has_object_and_adverb_guards() {
    for text in [
        "Developers call this a linter. Everyone else can call it a checker that will not let you ship.",
        "Founders call it hustle. Investors call it burn rate.",
    ] {
        finds(text, "S004");
    }
    for text in [
        "Call the office before nine. Call again if nobody answers.",
        "The client calls it once at startup. The server calls it again on reconnect.",
        "Some people call it a bug. Others call it a bug as well, and they are right because the spec says the field is optional.",
        "Engineers call this idempotency. It means a repeated request changes nothing.",
        "Customers call us daily. Partners call us weekly.",
        "The client calls it asynchronously. The server calls it directly.",
    ] {
        no_pairing(text);
    }
}

#[test]
fn ordinal_drumbeat_requires_a_sentence_opening_count_and_a_dropped_noun() {
    finds(
        "Four groups strip the AI accent. The fifth asks whether the line sells anything.",
        "S005",
    );
    for text in [
        "Four tests cover the parser. The fourth test is the slow one.",
        "Three files changed in this commit. The third one is generated, so ignore it.",
        "We ship four releases a year. The first ships in March.",
    ] {
        no_pairing(text);
    }
}

#[test]
fn noun_chiasmus_requires_indefinite_head_nouns_and_an_immediate_copula() {
    for text in [
        "A page that smells of it is a page they stop trusting.",
        "Fake proof is a sales failure before it is a writing failure.",
        "a sales failure before it is a writing failure",
    ] {
        finds(text, "S006");
    }
    for text in [
        "The default branch in that repo is the branch the workflow checks out.",
        "The first argument you pass is the argument the callback receives.",
        "The build that failed overnight is the build we shipped on Friday.",
        "The number in the contract is the number we hold you to.",
        "The only file that matters is the file the linter reads.",
        "A backup that is not restored is not a backup.",
        "A blue car is a blue truck.",
        "A page that works is the page we ship.",
        "The page that works is a page we ship.",
    ] {
        no_pairing(text);
    }
}

#[test]
fn markdown_boundaries_prevent_pairing() {
    for text in [
        "## Four things, done properly\nThe fifth asks whether it holds.",
        "- Developers call this a linter\n- Everyone else can call it a checker",
        "## Four things, done properly.\nThe fifth asks whether it holds.",
        "Developers call this a linter.\n## Everyone else can call it a checker.",
        "- Developers call this a linter.\n- Everyone else can call it a checker.",
        "1. Developers call this a linter.\n2. Everyone else can call it a checker.",
        "| Developers call this a linter. | Everyone else can call it a checker. |",
        "> Developers call this a linter.\n> Everyone else can call it a checker.",
        "Developers call this a linter.\n\nEveryone else can call it a checker.",
        "## A page that smells of it\nis a page they stop trusting.",
        "| A page that smells of it | is a page they stop trusting. |",
    ] {
        no_pairing(text);
    }
    finds(
        "- Founders call it hustle. Investors call it burn rate.",
        "S004",
    );
    finds(
        "Founders call it hustle.\nInvestors call it burn rate.",
        "S004",
    );
}

#[test]
fn stock_signoff() {
    finds(r#"Let me know if you have any other questions!"#, "P069");
    finds(r#"Let me know if you have any questions."#, "P069");
    finds(r#"Hope that helps!"#, "P069");
    assert!(!engine()
        .lint(r#"Let me know if the server fails."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P069"));
    assert!(!engine()
        .lint(r#"Send me the error log."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P069"));
}

#[test]
fn happy_to() {
    finds(r#"I'd be happy to add Sarah as a user."#, "P070");
    finds(r#"I would be happy to help."#, "P070");
    assert!(!engine()
        .lint(r#"I can add Sarah as a user."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P070"));
}

#[test]
fn performed_delight() {
    finds(r#"Thanks so much for the kind words."#, "P071");
    finds(r#"I am really glad you're enjoying it."#, "P071");
    finds(r#"I am really glad you are enjoying it."#, "P071");
    finds(r#"Glad that landed well!"#, "P071");
    assert!(!engine()
        .lint(r#"Thanks for the error log."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P071"));
    assert!(!engine()
        .lint(r#"I am glad the patch fixed the crash."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P071"));
}

#[test]
fn labelled_honest_note() {
    finds(
        r#"One honest note on cost: it will take another day."#,
        "P072",
    );
    finds(r#"One honest note: the cost has changed."#, "P072");
    assert!(!engine()
        .lint(r#"Note: costs exclude tax."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P072"));
    assert!(!engine()
        .lint(r#"Cost: 30 dollars."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P072"));
    assert!(!engine()
        .lint(r#"The parser reads one note on each line."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P072"));
}

#[test]
fn messaging_narration() {
    finds(r#"I wanted to reach out about the schedule."#, "P073");
    finds(r#"I just wanted to follow up on the schedule."#, "P073");
    assert!(!engine()
        .lint(r#"I wanted to reach the server before it shut down."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P073"));
    assert!(!engine()
        .lint(r#"I followed up with the vendor."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P073"));
}

#[test]
fn canned_reassurance() {
    finds(r#"Thank you for your patience."#, "P074");
    finds(r#"Rest assured, the patch is coming."#, "P074");
    assert!(!engine()
        .lint(r#"The patch ships today."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P074"));
}

#[test]
fn caveat_preface() {
    finds(r#"Just so you know, the API has changed."#, "P075");
    finds(r#"I should mention that the API has changed."#, "P075");
    assert!(!engine()
        .lint(r#"The API has changed."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P075"));
    assert!(!engine()
        .lint(r#"The docs should mention retries."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P075"));
}

#[test]
fn conditional_suggestion() {
    finds(r#"I would suggest shipping the patch today."#, "P076");
    assert!(!engine()
        .lint(r#"I suggest shipping the patch today."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P076"));
    assert!(!engine()
        .lint(r#"The result would suggest a race condition."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P076"));
}

#[test]
fn balanced_reason() {
    finds(
        r#"We delayed not because it was hard but because the tests failed."#,
        "P077",
    );
    assert!(!engine()
        .lint(r#"We delayed because the tests failed."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P077"));
    assert!(!engine()
        .lint(r#"It failed not because of memory. But because is also a conjunction."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P077"));
}

#[test]
fn corrective_source_framing() {
    finds(
        r#"The claims should be treated as religious tradition rather than as an archaeological explanation."#,
        "P078",
    );
    finds(
        r#"The report should be read as a hypothesis rather than a conclusion."#,
        "P078",
    );
    finds(
        r#"The statement should be understood as opinion rather than as evidence."#,
        "P078",
    );
    finds(
        r#"The account should be seen as a story rather than a record."#,
        "P078",
    );
    assert!(!engine()
        .lint(r#"We chose Postgres rather than MySQL for its JSON support."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P078"));
    assert!(!engine()
        .lint(r#"Use a queue rather than a shared counter."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P078"));
    assert!(!engine()
        .lint(r#"The value should be treated as a string."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P078"));
    assert!(!engine()
        .lint(r#"The value should be read as bytes. Use a buffer rather than a pointer."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P078"));
}

#[test]
fn does_not_establish() {
    finds(
        r#"It does not establish that Benjakitti has a particular contamination problem or groundwater configuration."#,
        "P079",
    );
    assert!(!engine()
        .lint(r#"The test establishes a connection."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P079"));
    for text in [
        "A correlation does not establish causation.",
        "Two counts should not be confused with each other here, the bed nights and the unique clients.",
    ] {
        assert!(!engine().lint(text).findings.iter().any(|f| f.rule_id.starts_with('P')), "{text}");
    }
    assert!(!engine()
        .lint(r#"We did not establish a connection."#)
        .findings
        .iter()
        .any(|f| f.rule_id == "P079"));
}

#[test]
fn remaining_pr14_body_and_patch_examples() {
    finds("X calls it A. Y calls it B.", "S004");
    for text in [
        "Six nails per shingle, every shingle.",
        "You get a written scope and a fixed number before anyone climbs a ladder.",
        "We do not do overlays. If the roof needs replacing, it gets stripped.",
        "The kickoff call ran forty minutes over, and the client paid the invoice early.",
        "Trusted, reliable and built to last.",
        "The fourth test is the slow one.",
    ] {
        no_pairing(text);
    }
}

#[test]
fn shapes_preserve_spans_and_support_articles_and_inflections() {
    finds("An error that ships is an error we own.", "S006");
    finds("A failed sales pitch is a failed writing pitch.", "S006");
    finds(
        "3 groups check the output. The 4th asks whether it works.",
        "S005",
    );
    finds(
        "Founders call them ideas. Investors call them costs.",
        "S004",
    );
    let text = "# Intro\n\nFounders call it hustle. Investors call it burn rate.";
    let report = engine().lint(text);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "S004")
        .unwrap();
    assert_eq!(
        &text[f.start..f.end],
        "Founders call it hustle. Investors call it burn rate."
    );
    no_pairing(&format!(
        "Founders call it {}. Investors call it {}.",
        "hustle".repeat(20),
        "cost".repeat(20)
    ));
}

#[test]
fn pr13_composite_reply_has_fixed_phrase_receipts() {
    let text = "Hi Jennifer! Thanks so much for the kind words about the dashboard, really glad you're enjoying it so far! To answer your questions: 1) Yes, I can absolutely update the header color to match your brand blue, I will get that changed today. 2) Of course! I'd be happy to add Sarah as a user, I will just need her email address. 3) Regarding the SMS piece, we are still on track and I am expecting to have it ready by the end of next week. Let me know if you have any other questions!";
    for id in ["P069", "P070", "P071"] {
        finds(text, id);
    }
    assert_eq!(
        engine()
            .lint(text)
            .findings
            .iter()
            .filter(|f| f.rule_id == "P071")
            .count(),
        2
    );
    let clean = "Header changes to brand blue today. Send me Sarah's email and I'll get her access. SMS is on track for end of next week.";
    assert!(engine().lint(clean).findings.is_empty());
    finds("I\u{2019}d be happy to help.", "P070");
    finds("Really glad you\u{2019}re enjoying it.", "P071");
}

#[test]
fn corrective_frames_do_not_cross_sentence_or_markdown_boundaries() {
    for text in [
        "We chose Postgres rather than MySQL for its JSON support.",
        "The result should be read as a count. Store it rather than recompute it.",
        "- It should be treated as a draft\n- Rather than delay, we sent it.",
        "| It should be treated as a draft | rather than a report |",
    ] {
        assert!(!engine()
            .lint(text)
            .findings
            .iter()
            .any(|f| matches!(f.rule_id.as_str(), "P078" | "P079")));
    }
}

#[test]
fn shape_rules_are_selected_and_overridden_by_packs() {
    let base = Pack::parse(tellem_core::BASE_PACK).unwrap();
    let override_pack = Pack::parse(
        r#"
[meta]
name = "override"
version = "0.1"
[[rules]]
id = "S004"
name = "owner-pairing"
kind = "shape"
shape = "relabel-pairing"
weight = 0.1
rationale = "owner receipt"
"#,
    )
    .unwrap();
    let e = Engine::from_packs(&[base, override_pack]).unwrap();
    let report = e.lint("Founders call it hustle. Investors call it burn rate.");
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "S004")
        .unwrap();
    assert_eq!(f.weight, 0.1);
    assert_eq!(f.rationale, "owner receipt");
    let empty = Engine::from_packs(&[]).unwrap();
    assert!(empty
        .lint("Founders call it hustle. Investors call it burn rate.")
        .findings
        .is_empty());
}

#[test]
fn relabel_pairing_spares_automatic_and_periodic_function_calls() {
    no_pairing("The client calls it automatically. The server calls it periodically.");
}

#[test]
fn relabel_pairing_accepts_people_as_third_person_objects() {
    finds(
        "Staff call her a leader. Clients call her a partner.",
        "S004",
    );
    finds(
        "Staff call him a leader. Clients call him a partner.",
        "S004",
    );
    no_pairing("Staff call you a leader. Clients call you a partner.");
}
