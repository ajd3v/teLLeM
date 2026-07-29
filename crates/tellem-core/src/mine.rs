//! mine: learn a per-family fingerprint from a labelled corpus.
//!
//! Features are word rates, selected by log-odds with an informative Dirichlet
//! prior (Monroe, Colaresi and Quinn 2008, the standard corpus-linguistics move
//! for "which words distinguish group A from the pooled background"). The z
//! score decides which words are worth keeping and it is also what a receipt
//! prints. Scoring itself is multinomial naive Bayes over the kept vocabulary,
//! chosen because every token's contribution can be printed. That constraint is
//! the point of the tool, so it outranks any accuracy a black box would buy.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

/// The rejection class. Text that looks like none of the catalogued models
/// lands here, and `who` reports no catalog match rather than naming the least
/// bad fit. Without it a five-way softmax must pick one of five, whatever it is
/// shown: on 30k human texts that produced a 45% false positive rate.
pub const UNMATCHED: &str = "unmatched";

/// One labelled corpus sample.
pub struct Sample<'a> {
    pub family: &'a str,
    pub text: &'a str,
}

/// What `mine` writes per family. Human-readable and diffable on purpose: the
/// catalog is the deliverable, and it stays useful when the classifier abstains.
#[derive(Serialize, Deserialize, Clone)]
pub struct Fingerprint {
    pub family: String,
    pub samples: usize,
    pub tokens: u64,
    /// logistic regression intercept for this family
    #[serde(default)]
    pub bias: f32,
    /// word -> evidence, sorted by word so a re-mine produces a clean diff
    pub features: BTreeMap<String, Feature>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Feature {
    /// logistic regression coefficient. Contribution to a call is weight*value,
    /// which is what a receipt prints.
    #[serde(default)]
    pub weight: f32,
    /// log-odds z vs the pooled background. Selection criterion and receipt.
    pub z: f32,
    /// occurrences per 1000 tokens in this family
    pub rate: f32,
    /// occurrences per 1000 tokens across the pooled corpus
    pub baseline: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Catalog {
    /// Corpus provenance, printed by `who` so a reader knows the closed set.
    pub source: String,
    /// Inverse document frequency per vocabulary feature. Defines the vocabulary
    /// and its column order for scoring.
    #[serde(default)]
    pub idf: BTreeMap<String, f32>,
    pub fingerprints: Vec<Fingerprint>,
}

impl Catalog {
    pub fn families(&self) -> Vec<&str> {
        self.fingerprints
            .iter()
            .map(|f| f.family.as_str())
            .collect()
    }
}

/// Lowercased word tokens. Deliberately crude: anything cleverer is a model of
/// English, and a model of English is a thing that can be wrong in ways nobody
/// can print.
pub fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty() && w.len() < 32)
        .map(|w| w.to_lowercase())
}

/// The full feature bag: words, word bigrams, punctuation, and layout habits.
/// Namespaced so a receipt can say which kind of evidence fired, and so a
/// bigram can never collide with a word. SPEC 3b asks for all four, and
/// punctuation profile in particular is the tell class the base pack is built
/// on, so dropping it would be strange.
pub fn tokens(text: &str) -> Vec<String> {
    let ws: Vec<String> = words(text).collect();
    let mut out: Vec<String> = Vec::with_capacity(ws.len() * 2 + 32);
    out.extend(ws.iter().map(|w| format!("w:{w}")));
    for pair in ws.windows(2) {
        out.push(format!("b:{} {}", pair[0], pair[1]));
    }
    for c in text.chars() {
        if !c.is_alphanumeric() && !c.is_whitespace() && c != '\'' {
            out.push(format!("p:{c}"));
        }
    }
    for line in text.lines() {
        let t = line.trim_start();
        let kind = if t.starts_with("```") {
            "fence"
        } else if t.starts_with("#") {
            "heading"
        } else if t.starts_with("- ") || t.starts_with("* ") {
            "bullet"
        } else if t.starts_with("**") {
            "boldlead"
        } else if t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains(". ") {
            "numlist"
        } else if t.is_empty() {
            "blank"
        } else {
            // Sentence-length habit, bucketed. Raw length is a confound, the
            // SHAPE of the distribution is the fingerprint.
            match t.split_whitespace().count() {
                0..=9 => "line_short",
                10..=24 => "line_mid",
                25..=49 => "line_long",
                _ => "line_para",
            }
        };
        out.push(format!("s:{kind}"));
    }
    out
}

#[derive(Default)]
struct Counts {
    total: u64,
    by_word: HashMap<String, u64>,
    samples: usize,
}

/// Mine a catalog. `top_k` features are kept per family, by |z|.
pub fn mine(samples: impl IntoIterator<Item = Sample<'static>>, top_k: usize) -> Catalog {
    mine_from(
        samples
            .into_iter()
            .map(|s| (s.family.to_string(), s.text.to_string())),
        top_k,
    )
}

/// Same, over owned pairs, so a caller can stream a corpus without lifetimes.
pub fn mine_from(samples: impl IntoIterator<Item = (String, String)>, top_k: usize) -> Catalog {
    let mut per_family: BTreeMap<String, Counts> = BTreeMap::new();
    let mut pooled = Counts::default();

    let mut doc_freq: HashMap<String, u64> = HashMap::new();
    for (family, text) in samples {
        let c = per_family.entry(family).or_default();
        c.samples += 1;
        pooled.samples += 1;
        let toks = tokens(&text);
        for w in &toks {
            c.total += 1;
            pooled.total += 1;
            *c.by_word.entry(w.clone()).or_insert(0) += 1;
            *pooled.by_word.entry(w.clone()).or_insert(0) += 1;
        }
        for w in toks.into_iter().collect::<std::collections::BTreeSet<_>>() {
            *doc_freq.entry(w).or_insert(0) += 1;
        }
    }

    // Informative Dirichlet prior: the pooled corpus itself, scaled. Rare words
    // then need far more evidence to look distinctive, which is exactly the
    // failure mode a plain frequency ratio has.
    let alpha0 = 1000.0_f64;
    let n = pooled.total as f64;

    let z_of = |c: &Counts, word: &str, y_iw: u64| -> f64 {
        let ni = c.total as f64;
        let y_w = pooled.by_word[word] as f64;
        let a_w = alpha0 * (y_w / n);
        let yi = y_iw as f64 + a_w;
        let yb = y_w + a_w;
        let delta = (yi / (ni + alpha0 - yi)).ln() - (yb / (n + alpha0 - yb)).ln();
        delta / (1.0 / yi + 1.0 / yb).sqrt()
    };

    // Each family nominates its top_k most distinctive words, and the catalog
    // vocabulary is the UNION. Every fingerprint then carries a probability for
    // every vocabulary word, including the ones it rarely uses, because "this
    // family almost never says moreover" is evidence too. Scoring only the
    // intersection would throw away nearly everything.
    let mut vocab: BTreeMap<String, ()> = BTreeMap::new();
    for c in per_family.values() {
        let mut scored: Vec<(f64, &String)> = c
            .by_word
            .iter()
            // A word seen a handful of times is noise, not a fingerprint.
            .filter(|(_, &y)| y >= 5)
            .map(|(word, &y)| (z_of(c, word, y).abs(), word))
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, w) in scored.into_iter().take(top_k) {
            vocab.insert(w.clone(), ());
        }
    }

    let fingerprints = per_family
        .iter()
        .map(|(family, c)| {
            let ni = c.total as f64;
            let features = vocab
                .keys()
                .map(|word| {
                    let y_iw = c.by_word.get(word).copied().unwrap_or(0);
                    let y_w = pooled.by_word[word] as f64;
                    let feature = Feature {
                        weight: 0.0,
                        z: z_of(c, word, y_iw) as f32,
                        rate: (y_iw as f64 / ni * 1000.0) as f32,
                        baseline: (y_w / n * 1000.0) as f32,
                    };
                    (word.clone(), feature)
                })
                .collect();
            Fingerprint {
                family: family.clone(),
                samples: c.samples,
                tokens: c.total,
                bias: 0.0,
                features,
            }
        })
        .collect();

    let n_docs = pooled.samples as f32;
    let idf = vocab
        .keys()
        .map(|w| {
            let df = doc_freq.get(w).copied().unwrap_or(0) as f32;
            (w.clone(), ((1.0 + n_docs) / (1.0 + df)).ln() + 1.0)
        })
        .collect();

    Catalog {
        source: String::new(),
        idf,
        fingerprints,
    }
}
