//! who: score text against the catalog, and refuse below threshold.
//!
//! The score is a dot product of the text's tf-idf vector with each family's
//! coefficients, so a single feature's contribution is `weight * value` and can
//! be printed. Confidence is the gap between the top two softmax posteriors,
//! NOT the raw score gap: raw scores grow with length, so thresholding on them
//! calls long text confidently and short text never, which is a length
//! threshold wearing a confidence costume.

use serde::Serialize;

use crate::mine::{Catalog, UNMATCHED};
use crate::train::vectorize;

#[derive(Serialize, Debug, Clone)]
pub struct Candidate {
    pub family: String,
    /// posterior probability, the five summing to 1
    pub probability: f32,
}

#[derive(Serialize, Debug, Clone)]
pub struct Receipt {
    /// namespaced feature: w: word, b: bigram, p: punctuation, s: layout
    pub feature: String,
    /// how far this feature pushed the winner ahead of the runner-up
    pub contribution: f32,
    /// occurrences per 1000 tokens in the winning family
    pub rate: f32,
    /// occurrences per 1000 tokens across the whole catalog corpus
    pub baseline: f32,
}

#[derive(Serialize, Debug, Clone)]
pub struct Attribution {
    pub ranked: Vec<Candidate>,
    /// top posterior minus runner-up. The quantity the threshold gates on.
    pub confidence: f32,
    /// vocabulary features the text actually hit
    pub matched: usize,
    /// Some(family) only above threshold. None means insufficient signal.
    pub call: Option<String>,
    /// Why the call went the way it did, biggest contributors first.
    pub receipts: Vec<Receipt>,
    /// Printed on every result: the closed set this answer is relative to.
    pub catalog: Vec<String>,
}

impl Catalog {
    /// Attribute `text`, calling a family only when the posterior gap clears
    /// `min_confidence` and enough features matched to mean anything.
    pub fn who(&self, text: &str, min_confidence: f32, min_matched: usize) -> Attribution {
        let index = self.index();
        let idf: Vec<f32> = self.idf.values().copied().collect();
        let x = vectorize(text, &index, &idf);
        let names: Vec<&String> = self.idf.keys().collect();

        let mut scores: Vec<f32> = self
            .fingerprints
            .iter()
            .map(|f| {
                f.bias
                    + x.iter()
                        .map(|&(i, v)| f.features.get(names[i]).map_or(0.0, |c| c.weight) * v)
                        .sum::<f32>()
            })
            .collect();

        let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0.0;
        for s in &mut scores {
            *s = (*s - max).exp();
            sum += *s;
        }
        let mut ranked: Vec<(usize, Candidate)> = self
            .fingerprints
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    i,
                    Candidate {
                        family: f.family.clone(),
                        probability: if sum > 0.0 { scores[i] / sum } else { 0.0 },
                    },
                )
            })
            .collect();
        ranked.sort_by(|a, b| b.1.probability.total_cmp(&a.1.probability));

        let confidence = match ranked.as_slice() {
            [a, b, ..] => a.1.probability - b.1.probability,
            _ => 0.0,
        };
        // Ranking the rejection class first is a refusal, at any confidence.
        let call = (x.len() >= min_matched
            && confidence >= min_confidence
            && !ranked.is_empty()
            && ranked[0].1.family != UNMATCHED)
            .then(|| ranked[0].1.family.clone());

        let receipts = match (&call, ranked.len()) {
            (Some(_), n) if n >= 2 => self.receipts_for(ranked[0].0, ranked[1].0, &x, &names),
            _ => Vec::new(),
        };

        Attribution {
            ranked: ranked.into_iter().map(|(_, c)| c).collect(),
            confidence,
            matched: x.len(),
            call,
            receipts,
            catalog: self.families().into_iter().map(String::from).collect(),
        }
    }

    fn receipts_for(
        &self,
        winner: usize,
        runner: usize,
        x: &[(usize, f32)],
        names: &[&String],
    ) -> Vec<Receipt> {
        let (w, r) = (&self.fingerprints[winner], &self.fingerprints[runner]);
        let mut rows: Vec<Receipt> = x
            .iter()
            .filter_map(|&(i, v)| {
                let wf = w.features.get(names[i])?;
                let rf = r.features.get(names[i])?;
                Some(Receipt {
                    feature: names[i].clone(),
                    contribution: (wf.weight - rf.weight) * v,
                    rate: wf.rate,
                    baseline: wf.baseline,
                })
            })
            .collect();
        rows.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
        rows.truncate(8);
        rows
    }
}
