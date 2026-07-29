//! Attribution on a synthetic corpus. The real numbers come from `tellem eval`
//! against a 51k-sample corpus that is far too big to check in, so this guards
//! the mechanics instead: does a fitted catalog separate two obviously
//! different writers, refuse on text it has no opinion about, and print a
//! receipt that names the word responsible.

use tellem_core::train::fit;

/// Two writers with distinct habits and a shared topic, so the separation has
/// to come from style rather than subject.
fn corpus() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for i in 0..60 {
        out.push((
            "alpha".to_string(),
            format!(
                "Moreover the system delivers robust throughput. Furthermore it \
                 scales. Moreover the run {i} completed. Furthermore latency held."
            ),
        ));
        out.push((
            "beta".to_string(),
            format!(
                "The system got faster. It scales fine. Run {i} finished clean. \
                 Latency held steady, no drama."
            ),
        ));
    }
    out
}

#[test]
fn separates_two_writers_and_shows_its_work() {
    let cat = fit(&corpus(), 200, 8);
    assert_eq!(cat.families(), vec!["alpha", "beta"]);

    let a = cat.who(
        "Moreover the pipeline is robust. Furthermore the throughput scales.",
        0.5,
        1,
    );
    assert_eq!(a.call.as_deref(), Some("alpha"), "{:?}", a.ranked);
    assert!(
        a.receipts.iter().any(|r| r.feature.contains("moreover")),
        "no receipt named the word that decided it: {:?}",
        a.receipts
    );

    let b = cat.who("The run finished clean, no drama.", 0.5, 1);
    assert_eq!(b.call.as_deref(), Some("beta"), "{:?}", b.ranked);
}

#[test]
fn refuses_rather_than_guesses() {
    let cat = fit(&corpus(), 200, 8);
    // Nothing in the vocabulary, so there is no evidence either way.
    let a = cat.who("zzz qqq", 0.5, 4);
    assert_eq!(
        a.call, None,
        "called a family on no evidence: {:?}",
        a.ranked
    );
    assert!(a.receipts.is_empty());
    // Every result names the closed set it is relative to.
    assert_eq!(a.catalog, vec!["alpha", "beta"]);
}

#[test]
fn posteriors_are_a_distribution() {
    let cat = fit(&corpus(), 200, 8);
    let a = cat.who("Moreover the system is robust.", 0.0, 0);
    let total: f32 = a.ranked.iter().map(|c| c.probability).sum();
    assert!((total - 1.0).abs() < 1e-3, "posteriors sum to {total}");
    assert!(a.confidence >= 0.0 && a.confidence <= 1.0);
}

#[test]
fn fitting_is_deterministic() {
    // Same corpus in, same catalog out, or a published confusion matrix means
    // nothing and a catalog diff is noise.
    let (a, b) = (fit(&corpus(), 200, 8), fit(&corpus(), 200, 8));
    assert_eq!(a.idf.len(), b.idf.len());
    for (x, y) in a.fingerprints.iter().zip(&b.fingerprints) {
        assert_eq!(x.bias, y.bias);
        for (k, v) in &x.features {
            assert_eq!(v.weight, y.features[k].weight, "{k} drifted between fits");
        }
    }
}
