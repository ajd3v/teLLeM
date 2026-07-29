//! Multinomial logistic regression over the mined vocabulary.
//!
//! Why not stay with naive Bayes: on the held-out eval NB tops out at 72.7%
//! forced choice and clears the 95% precision floor only at 34% coverage,
//! because it assumes features are independent and word bigrams are anything
//! but. Logistic regression drops that assumption and roughly doubles coverage
//! at the same floor.
//!
//! Why this is still explainable, which is the invariant: the score is a plain
//! dot product, so one feature's contribution to a call is `weight * value` and
//! prints exactly like a naive Bayes term did. Nothing here is a black box.

use std::collections::BTreeMap;

use crate::mine::{tokens, Catalog};

/// Sublinear tf, idf, L2 normalised. The normalisation is what stops response
/// length from dominating: a long sample and a short one produce vectors of the
/// same magnitude, so the classifier reads style rather than word count.
pub fn vectorize(text: &str, index: &BTreeMap<String, usize>, idf: &[f32]) -> Vec<(usize, f32)> {
    let mut counts: BTreeMap<usize, f32> = BTreeMap::new();
    for t in tokens(text) {
        if let Some(&i) = index.get(&t) {
            *counts.entry(i).or_insert(0.0) += 1.0;
        }
    }
    let mut v: Vec<(usize, f32)> = counts
        .into_iter()
        .map(|(i, c)| (i, (1.0 + c.ln()) * idf[i]))
        .collect();
    let norm = v.iter().map(|(_, x)| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for (_, x) in &mut v {
            *x /= norm;
        }
    }
    v
}

pub struct Trained {
    /// [class][feature]
    pub weights: Vec<Vec<f32>>,
    pub bias: Vec<f32>,
}

/// AdaGrad on the softmax cross-entropy, with L2. AdaGrad because the features
/// are wildly different scales (a punctuation mark fires in every sample, a
/// distinctive bigram in a handful) and a per-feature learning rate handles
/// that without tuning a schedule.
pub fn train(
    docs: &[(usize, Vec<(usize, f32)>)],
    n_features: usize,
    n_classes: usize,
    epochs: usize,
    lr: f32,
    l2: f32,
) -> Trained {
    let mut w = vec![vec![0.0f32; n_features]; n_classes];
    let mut b = vec![0.0f32; n_classes];
    let mut g2 = vec![vec![1e-8f32; n_features]; n_classes];
    let mut gb2 = vec![1e-8f32; n_classes];
    // Deterministic shuffle: same catalog from the same corpus, every run.
    let mut order: Vec<usize> = (0..docs.len()).collect();

    for epoch in 0..epochs {
        let mut seed = 0x9e3779b97f4a7c15u64 ^ epoch as u64;
        for i in (1..order.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            order.swap(i, (seed % (i as u64 + 1)) as usize);
        }
        for &d in &order {
            let (label, x) = &docs[d];
            let mut scores: Vec<f32> = (0..n_classes)
                .map(|c| b[c] + x.iter().map(|&(i, v)| w[c][i] * v).sum::<f32>())
                .collect();
            let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let mut sum = 0.0;
            for s in &mut scores {
                *s = (*s - max).exp();
                sum += *s;
            }
            for c in 0..n_classes {
                let p = scores[c] / sum;
                let err = p - if c == *label { 1.0 } else { 0.0 };
                if err.abs() < 1e-7 {
                    continue;
                }
                gb2[c] += err * err;
                b[c] -= lr * err / gb2[c].sqrt();
                for &(i, v) in x {
                    let g = err * v + l2 * w[c][i];
                    g2[c][i] += g * g;
                    w[c][i] -= lr * g / g2[c][i].sqrt();
                }
            }
        }
    }
    Trained {
        weights: w,
        bias: b,
    }
}

impl Catalog {
    /// Feature name -> column, in the catalog's own stable order.
    pub fn index(&self) -> BTreeMap<String, usize> {
        self.idf
            .keys()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect()
    }
}

/// Mine the descriptive catalog, then fit the classifier onto it. One artifact:
/// the human-readable atlas and the thing that does the scoring are the same
/// file, so a coefficient can never drift from the evidence beside it.
/// Vocabulary size is set by `top_k` at mining time. Pruning after training,
/// by how hard the classifier pulls on each feature, was tried and measurably
/// LOST to plain |z| selection at the same final size (86.5% against 88.9%
/// forced choice). Distributed weak evidence carries this task, so the code
/// that concentrated on strong features is gone rather than kept "just in case".
pub fn fit(samples: &[(String, String)], top_k: usize, epochs: usize) -> Catalog {
    let mut catalog = crate::mine::mine_from(samples.iter().cloned(), top_k);
    fit_once(&mut catalog, samples, epochs);
    // Five significant figures. An f32 serialises as its f64 expansion
    // otherwise, which triples the file and makes every diff unreadable.
    for v in catalog.idf.values_mut() {
        *v = round5(*v);
    }
    for fp in &mut catalog.fingerprints {
        fp.bias = round5(fp.bias);
        for f in fp.features.values_mut() {
            f.weight = round5(f.weight);
            f.z = round5(f.z);
            f.rate = round5(f.rate);
            f.baseline = round5(f.baseline);
        }
    }
    catalog
}

fn round5(x: f32) -> f32 {
    (x * 1e5).round() / 1e5
}

fn fit_once(catalog: &mut Catalog, samples: &[(String, String)], epochs: usize) {
    let index = catalog.index();
    let idf: Vec<f32> = catalog.idf.values().copied().collect();
    let classes: Vec<String> = catalog
        .fingerprints
        .iter()
        .map(|f| f.family.clone())
        .collect();

    let docs: Vec<(usize, Vec<(usize, f32)>)> = samples
        .iter()
        .map(|(family, text)| {
            let c = classes.iter().position(|f| f == family).unwrap();
            (c, vectorize(text, &index, &idf))
        })
        .collect();

    let trained = train(&docs, index.len(), classes.len(), epochs, 0.5, 1e-6);
    let names: Vec<String> = catalog.idf.keys().cloned().collect();
    for (c, fp) in catalog.fingerprints.iter_mut().enumerate() {
        fp.bias = trained.bias[c];
        for (i, name) in names.iter().enumerate() {
            if let Some(f) = fp.features.get_mut(name) {
                f.weight = trained.weights[c][i];
            }
        }
    }
}
