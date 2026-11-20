//! Coverage for the handoff gaps, tested through the public lint interface.
use tellem_core::{Engine, Pack};

const VOICE_PACK: &str = include_str!("fixtures/gainful-voice.toml");

fn engine(voice: bool) -> Engine {
    let mut packs = vec![Pack::parse(tellem_core::BASE_PACK).unwrap()];
    if voice {
        packs.push(Pack::parse(VOICE_PACK).unwrap());
    }
    Engine::from_packs(&packs).unwrap()
}

fn finds(text: &str, id: &str, voice: bool) {
    let e = engine(voice);
    let report = e.lint(text);
    let finding = report
        .findings
        .iter()
        .find(|f| f.rule_id == id)
        .unwrap_or_else(|| panic!("missing {id} on {text:?}, got {:?}", report.findings));
    assert!(!finding.rationale.is_empty());
    if matches!(id, "G008" | "G009" | "G010" | "G011")
        || id.starts_with('T') && id >= "T057"
        || id.starts_with('P') && id >= "P048"
    {
        assert!(!finding.fixable, "new rule {id} must remain flag-only");
    }
    for masked in [format!("`{text}`"), format!("<code>{text}</code>")] {
        assert!(
            !e.lint(&masked).findings.iter().any(|f| f.rule_id == id),
            "{masked}"
        );
    }
    let fixed = e.fix(text);
    let after = e.lint(&fixed);
    assert!(
        after.total_weight <= report.total_weight + f32::EPSILON,
        "weight rose on {text:?}, fixed {fixed:?}"
    );
    assert!(
        after.findings.iter().all(|f| !f.fixable),
        "fixable finding survived on {text:?}"
    );
    assert_eq!(finding.rationale.lines().count(), 1);
    assert!(finding.start < finding.end);
    assert!(text.get(finding.start..finding.end).is_some());
}

macro_rules! gap {
    ($name:ident, $id:literal, $sample:literal, $voice:literal) => {
        #[test]
        fn $name() {
            finds($sample, $id, $voice);
        }
    };
}

gap!(mirrors_g004, "G004", "My r\u{e9}sum\u{e9}.", true);

gap!(mirrors_g005, "G005", "R\u{e9}sum\u{e9} attached.", true);

gap!(
    state_of_the_art,
    "T057",
    "Our state-of-the-art approach.",
    false
);

gap!(
    transformation,
    "T058",
    "The digital transformation changed the workflow.",
    false
);

gap!(innovate, "T059", "We innovate.", false);

gap!(paradigm, "T060", "This is a new paradigm.", false);

gap!(bespoke, "T061", "Our bespoke approach.", false);

gap!(
    beacon,
    "T062",
    "The product is a beacon of progress.",
    false
);

gap!(unparalleled, "T063", "Our unparalleled approach.", false);

gap!(supercharge, "T064", "Supercharge your workflow.", false);

gap!(turbocharge, "T065", "Turbocharge your workflow.", false);

gap!(effortless, "T066", "Our effortless approach.", false);

gap!(next_level, "T067", "Our next-level approach.", false);

gap!(compelling, "T068", "Our compelling approach.", false);

gap!(intuitive, "T069", "Our intuitive approach.", false);

gap!(world_class, "T070", "Our world-class approach.", false);

gap!(best_in_class, "T071", "Our best-in-class approach.", false);

gap!(
    whether_you_are,
    "P048",
    "Whether you're a startup or an enterprise, we can help.",
    false
);

gap!(
    more_than_just,
    "P049",
    "It is more than just storage.",
    false
);

gap!(
    comes_in,
    "P050",
    "That's where our product comes in.",
    false
);

gap!(say_goodbye, "P051", "Say goodbye to manual work.", false);

gap!(imagine_a, "P052", "Imagine a simpler day.", false);

gap!(
    key_or_truth,
    "P053",
    "The key is understanding the process.",
    false
);

gap!(helps_you_to, "P054", "It helps you to finish.", false);

gap!(stacked_hedging, "P055", "It could potentially work.", false);

gap!(intensifier_padding, "P056", "It is very unique.", false);

gap!(
    heres_the_thing,
    "P057",
    "Here's the thing, it works.",
    false
);

gap!(call_to_action, "P058", "Ready to get started?", false);

gap!(
    self_answering_question,
    "P059",
    "The result? Fewer errors.",
    false
);

gap!(
    hyphenated_compounds,
    "P060",
    "We offer well-tested, production-ready, battle-tested and user-friendly software.",
    false
);

gap!(crafted, "P061", "It was crafted for you.", false);

gap!(curated, "P062", "A carefully curated collection.", false);

gap!(navigate_the, "P063", "Navigate the complexities.", false);

gap!(
    in_the_world_of,
    "P064",
    "In the world of software, changes happen.",
    false
);

gap!(lets_dive, "P065", "Let's dive into it.", false);

gap!(buckle_up, "P066", "Buckle up for the details.", false);

gap!(secret_sauce, "P067", "The secret sauce is caching.", false);

gap!(level_up, "P068", "Level up your work.", false);

gap!(owner_unlock, "G008", "It unlocks insights.", true);

gap!(
    promotional_triplet,
    "G009",
    "We help you to streamline workflows, boost productivity, and drive growth.",
    true
);

gap!(
    not_just_but,
    "G010",
    "It is not just a tool but a partner.",
    true
);

gap!(
    contracted_contrast,
    "G010",
    "It isn't just storage, it's a partner.",
    true
);

gap!(
    plain_contrast,
    "G010",
    "It is not a tool, it is a partner.",
    true
);

gap!(rhetorical_filler, "G011", "Why does this matter?", true);

gap!(
    marketing_contrast,
    "G010",
    "Our platform doesn't just store data, it unlocks insights.",
    true
);

#[test]
fn real_inflections_and_spelling_variants() {
    for (id, text, count) in [
        ("T057", "We use state of the art tools.", 1),
        (
            "T058",
            "These business transformations changed the workflow.",
            1,
        ),
        (
            "T059",
            "She innovates. We innovated by innovating. Our innovations were innovative.",
            5,
        ),
        (
            "T059",
            "That innovation changed how we work innovatively.",
            2,
        ),
        ("T060", "We promise new paradigms.", 1),
        ("T061", "The BESPOKE package arrived.", 1),
        ("T062", "These products are beacons of progress.", 1),
        ("T063", "They claim UNPARALLELED results.", 1),
        (
            "T064",
            "It supercharges reporting. We supercharged it by supercharging the parser.",
            3,
        ),
        (
            "T065",
            "It turbocharges reporting. We turbocharged it by turbocharging the parser.",
            3,
        ),
        ("T066", "It runs effortlessly.", 1),
        ("T067", "Take it to the next level.", 1),
        ("T068", "It presents the idea compellingly.", 1),
        ("T069", "It works intuitively.", 1),
        ("T070", "A world class service.", 1),
        ("T071", "They call it best in class.", 1),
        (
            "P048",
            "Whether you are a student or a teacher, it works.",
            1,
        ),
        ("P050", "That is where the tools come in.", 1),
        (
            "P051",
            "She says goodbye to delays. We said goodbye to delays by saying goodbye to paperwork.",
            3,
        ),
        ("P052", "Imagine an easier morning.", 1),
        ("P053", "The truth is simple.", 1),
        (
            "P054",
            "We help you to finish. It helped you to finish by helping you to focus.",
            3,
        ),
        ("P055", "It might possibly help. It may perhaps work.", 2),
        ("P056", "It is absolutely perfect.", 1),
        ("P057", "Here is the thing, the tests pass.", 1),
        ("P059", "The outcome? Fewer errors.", 1),
        (
            "P063",
            "She navigates the complexities. We navigated the challenges by navigating the maze.",
            3,
        ),
        ("P065", "Let us dive into the details.", 1),
        (
            "P068",
            "She levels up. We leveled up by leveling up our skills.",
            3,
        ),
        ("P068", "We levelled up by levelling up our skills.", 2),
        (
            "G008",
            "We unlock insights. It unlocks insights. We unlocked insights by unlocking reports.",
            4,
        ),
        ("G009", "The product is fast, scalable and reliable.", 1),
        ("G010", "It is not just storage, but a partner.", 1),
        ("G010", "It is not a bug, but a feature.", 1),
        (
            "G010",
            "It isn\u{2019}t just storage, it\u{2019}s a partner.",
            1,
        ),
        ("G011", "Why is this important?", 1),
    ] {
        finds(text, id, true);
        assert_eq!(
            engine(true)
                .lint(text)
                .findings
                .iter()
                .filter(|f| f.rule_id == id)
                .count(),
            count,
            "{id} on {text:?}"
        );
    }
}

#[test]
fn intentional_narrow_cases_stay_narrow() {
    let e = engine(false);
    for (text, id) in [
        ("Unlock the mutex.", "P027"),
        ("This could revolutionize storage.", "P029"),
        ("Foster care needs funding.", "T051"),
        ("In today's meeting, we wrote the plan.", "P003"),
        ("A fast-paced game.", "P003"),
        ("Dive in when you are ready.", "P020"),
        ("Elevated privileges are required.", "T052"),
    ] {
        assert!(
            !e.lint(text).findings.iter().any(|f| f.rule_id == id),
            "{text}"
        );
    }
    for (text, id) in [
        ("Unlock the potential.", "P027"),
        ("It will revolutionize the way we work.", "P029"),
        ("It fosters collaboration.", "T051"),
        ("We fostered collaboration.", "T051"),
        ("We are fostering collaboration.", "T051"),
        ("It elevates experiences.", "T052"),
        ("We are elevating experiences.", "T052"),
        ("In today's fast-paced world, we write code.", "P003"),
        ("We dive into the details.", "P020"),
    ] {
        finds(text, id, false);
    }
}

#[test]
fn factual_lists_and_ordinary_questions_are_spared() {
    let e = engine(true);
    for text in [
        "I build data, backend and web systems.",
        "Hired, trained and managed seven people over five years.",
        "Python, SQL, TypeScript, C++, Rust, PHP, JavaScript.",
        "We run well-tested, production-ready and user-friendly tools.",
        "We run well-tested and production-ready tools. They are battle-tested and user-friendly.",
        "We run state-of-the-art software.",
        "The result was fewer errors.",
        "Why does the server crash?",
        "What does this function return?",
        "This is not a tool. It is a partner.",
        "It is not a tool\nwe use, it is a partner.",
    ] {
        assert!(
            !e.lint(text).findings.iter().any(|f| matches!(
                f.rule_id.as_str(),
                "G009" | "G010" | "G011" | "P059" | "P060"
            )),
            "{text}"
        );
    }
}

#[test]
fn strict_rules_are_voice_only() {
    let e = engine(false);
    for text in [
        "It unlocks insights.",
        "Fast, scalable, and reliable.",
        "It is not a tool, it is a partner.",
        "Why does this matter?",
    ] {
        assert!(!e
            .lint(text)
            .findings
            .iter()
            .any(|f| f.rule_id.starts_with('G')));
    }
}

const VOICE_PASSAGE: &str = "I built the pipeline \u{2014} it runs nightly. The result was clear: reports got faster; nobody noticed. It is not a tool, it is a partner. Why does this matter? I utilized myriad intricate methods and I am passionate about my r\u{00e9}sum\u{00e9}. It's worth noting that I hit the ground running.\n";

const MARKETING_PASSAGE: &str = "In today's fast-paced world, our robust platform doesn't just store data, it unlocks insights. Whether you're a startup or an enterprise, we help you to streamline workflows, boost productivity, and drive growth. Trusted by 10,000+ happy customers.\n";

fn covers(text: &str, expected: &[(&str, &str)]) {
    let report = engine(true).lint(text);
    for &(id, evidence) in expected {
        let start = text.find(evidence).unwrap();
        let end = start + evidence.len();
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule_id == id && f.start < end && start < f.end),
            "missing {id} on {evidence:?}, got {:?}",
            report.findings
        );
    }
}

#[test]
fn all_twelve_voice_breaks_have_receipts() {
    covers(
        VOICE_PASSAGE,
        &[
            ("T001", "\u{2014}"),
            ("G003", ":"),
            ("G002", ";"),
            ("G010", "not a tool, it is a partner"),
            ("G011", "Why does this matter?"),
            ("T012", "utilized"),
            ("T025", "myriad"),
            ("T022", "intricate"),
            ("T053", "passionate"),
            ("G004", "r\u{e9}sum\u{e9}"),
            ("P004", "It's worth noting that"),
            ("P045", "hit the ground running"),
        ],
    );
    assert_eq!(engine(true).lint(VOICE_PASSAGE).findings.len(), 13);
}

#[test]
fn full_marketing_passage_has_receipts() {
    covers(
        MARKETING_PASSAGE,
        &[
            ("P003", "In today's fast-paced world"),
            ("T055", "robust"),
            ("G010", "doesn't just store data, it unlocks"),
            ("G008", "unlocks"),
            ("P048", "Whether you're a startup or an enterprise"),
            ("P054", "help you to"),
            ("T018", "streamline"),
            (
                "G009",
                "streamline workflows, boost productivity, and drive growth",
            ),
            ("G007", "10,000"),
        ],
    );
    assert_eq!(engine(true).lint(MARKETING_PASSAGE).findings.len(), 9);
}

fn literal_prose_is_clean(sentences: &[&str]) {
    let e = engine(false);
    for text in sentences {
        let report = e.lint(text);
        assert!(
            report.findings.is_empty(),
            "literal prose flagged on {text:?}: {:?}",
            report.findings
        );
    }
}

#[test]
fn technical_transformations_are_spared() {
    literal_prose_is_clean(&[
        "Run the matrix transformation on the GPU.",
        "Built ETL pipelines that run data transformations in dbt and Python.",
        "The transformation step validates every row before load.",
    ]);
    for text in [
        "Our digital transformation changed the business.",
        "The business transformation starts today.",
        "This is a transformational product.",
        "We deliver digital transformations.",
        "We deliver business transformations.",
    ] {
        finds(text, "T058", false);
    }
    // T038 already covers this adjective. Keep one receipt for it.
    finds("This is a transformative product.", "T038", false);
}

#[test]
fn programming_paradigms_are_spared() {
    literal_prose_is_clean(&[
        "The script uses a functional programming paradigm.",
        "The language supports several programming paradigms.",
    ]);
    for text in [
        "This is a paradigm shift.",
        "This is a new paradigm.",
        "We promise paradigm shifts.",
        "We promise new paradigms.",
    ] {
        finds(text, "T060", false);
    }
}

#[test]
fn literal_beacons_are_spared() {
    literal_prose_is_clean(&[
        "Each store has a Bluetooth beacon by the door.",
        "The page sends a web beacon after loading.",
        "The receiver detects Bluetooth beacons.",
    ]);
    for text in [
        "The product is a beacon of progress.",
        "These products are beacons of progress.",
    ] {
        finds(text, "T062", false);
    }
}

#[test]
fn curated_datasets_are_spared() {
    literal_prose_is_clean(&[
        "We trained the model on a curated dataset of 4,000 labeled records.",
    ]);
    for text in [
        "Our carefully curated collection is ready.",
        "Our expertly curated collection is ready.",
        "We offer a curated experience.",
        "We offer curated experiences.",
        "Browse our curated selection.",
        "Browse our curated selections.",
    ] {
        finds(text, "P062", false);
    }
}

#[test]
fn literal_navigation_is_spared() {
    literal_prose_is_clean(&[
        "Click the menu to navigate the settings page.",
        "She navigates the menu.",
        "We navigated the settings page.",
        "The user is navigating the menu.",
    ]);
    for text in [
        "Navigate the complexities of the process.",
        "Navigate the landscape.",
        "Navigate the challenges ahead.",
        "Navigate the world of business.",
        "Navigate the waters of change.",
        "Navigate the maze of decisions.",
        "Navigate the intricacies of the process.",
        "She navigates the complexities.",
        "We navigated the challenges.",
        "We are navigating the maze.",
    ] {
        finds(text, "P063", false);
    }
}

#[test]
fn stored_keys_are_spared() {
    literal_prose_is_clean(&[
        "The key is stored in the vault and rotated monthly.",
        "The key is simple_key in the configuration.",
        "The key is token_id in the request.",
    ]);
    for text in [
        "The truth is we need more time.",
        "The key is to start early.",
        "The key is that we start early.",
        "The key is simple.",
        "The key is understanding the process.",
    ] {
        finds(text, "P053", false);
    }
}

#[test]
fn technical_passage_retains_requested_hedging_and_state_of_the_art_rules() {
    let e = engine(false);
    // The follow-up explicitly keeps these two rules unchanged.
    for (text, id) in [
        (
            "The fix may possibly affect older clients, so we ship it behind a flag.",
            "P055",
        ),
        (
            "State of the art results on this benchmark come from larger models.",
            "T057",
        ),
    ] {
        let report = e.lint(text);
        let ids: Vec<_> = report.findings.iter().map(|f| f.rule_id.as_str()).collect();
        assert_eq!(ids, [id], "{text}");
    }
}
