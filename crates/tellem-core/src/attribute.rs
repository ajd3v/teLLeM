//! who: score text against the catalog, and refuse below the margin.
//!
//! Multinomial naive Bayes over the mined vocabulary, scored as a MEAN log
//! probability per matched token so a long text and a short one produce
//! comparable margins. Without that normalization the threshold would really be
//! a length threshold, and the corpus shows response length varies more between
//! model families than almost anything else.

use serde::Serialize;

use crate::mine::{tokens, Catalog};

#[derive(Serialize, Debug, Clone)]
pub struct Candidate {
    pub family: String,
    /// mean log P(token | family) over tokens the catalog knows
    pub score: f32,
}

#[derive(Serialize, Debug, Clone)]
pub struct Receipt {
    pub word: String,
    /// how far this word pushed the winner ahead of the runner-up
    pub contribution: f32,
    /// occurrences per 1000 tokens in the winning family
    pub rate: f32,
    /// occurrences per 1000 tokens across the whole catalog corpus
    pub baseline: f32,
    pub count: usize,
}

#[derive(Serialize, Debug, Clone)]
pub struct Attribution {
    pub ranked: Vec<Candidate>,
    /// winner score minus runner-up score. The quantity the threshold gates on.
    pub margin: f32,
    /// tokens that matched the catalog vocabulary
    pub matched: usize,
    /// Some(family) only above threshold. None means insufficient signal.
    pub call: Option<String>,
    /// Why the call went the way it did, biggest contributors first.
    pub receipts: Vec<Receipt>,
    /// Printed on every result: the closed set this answer is relative to.
    pub catalog: Vec<String>,
}

impl Catalog {
    /// Attribute `text`, calling a family only when the margin clears
    /// `min_margin` and enough tokens matched to mean anything.
    pub fn who(&self, text: &str, min_margin: f32, min_matched: usize) -> Attribution {
        let words: Vec<String> = tokens(text);
        let mut totals = vec![0.0f64; self.fingerprints.len()];
        let mut counts = vec![0usize; self.fingerprints.len()];
        // per family, per word: summed log prob, for the receipts
        let mut per_word: Vec<(String, usize, Vec<f32>)> = Vec::new();

        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for w in &words {
            if let Some(&i) = seen.get(w) {
                per_word[i].1 += 1;
                continue;
            }
            // A word only counts when every family has an opinion about it,
            // otherwise a missing entry silently becomes evidence.
            let vals: Option<Vec<f32>> = self
                .fingerprints
                .iter()
                .map(|f| f.features.get(w).map(|x| x.log_prob))
                .collect();
            if let Some(vals) = vals {
                seen.insert(w.clone(), per_word.len());
                per_word.push((w.clone(), 1, vals));
            }
        }

        for (_, n, vals) in &per_word {
            for (i, v) in vals.iter().enumerate() {
                totals[i] += (*v as f64) * (*n as f64);
                counts[i] += n;
            }
        }

        let matched = counts.first().copied().unwrap_or(0);
        let mut ranked: Vec<Candidate> = self
            .fingerprints
            .iter()
            .enumerate()
            .map(|(i, f)| Candidate {
                family: f.family.clone(),
                // Summed, not averaged. The margin is then a log Bayes factor,
                // so a long sample earns confidence a short one has not: short
                // text abstains by construction rather than by special case.
                score: if matched == 0 {
                    f32::NEG_INFINITY
                } else {
                    totals[i] as f32
                },
            })
            .collect();
        ranked.sort_by(|a, b| b.score.total_cmp(&a.score));

        let margin = match ranked.as_slice() {
            [a, b, ..] => a.score - b.score,
            _ => 0.0,
        };
        let call = (matched >= min_matched && margin >= min_margin && !ranked.is_empty())
            .then(|| ranked[0].family.clone());

        let receipts = call
            .as_ref()
            .map(|winner| self.receipts_for(winner, &ranked, &per_word))
            .unwrap_or_default();

        Attribution {
            ranked,
            margin,
            matched,
            call,
            receipts,
            catalog: self.families().into_iter().map(String::from).collect(),
        }
    }

    fn receipts_for(
        &self,
        winner: &str,
        ranked: &[Candidate],
        per_word: &[(String, usize, Vec<f32>)],
    ) -> Vec<Receipt> {
        let idx = |name: &str| self.fingerprints.iter().position(|f| f.family == name);
        let (Some(wi), Some(ri)) = (idx(winner), ranked.get(1).and_then(|c| idx(&c.family))) else {
            return Vec::new();
        };
        let fp = &self.fingerprints[wi];
        let mut rows: Vec<Receipt> = per_word
            .iter()
            .map(|(word, n, vals)| {
                let f = &fp.features[word];
                Receipt {
                    word: word.clone(),
                    contribution: (vals[wi] - vals[ri]) * *n as f32,
                    rate: f.rate,
                    baseline: f.baseline,
                    count: *n,
                }
            })
            .collect();
        rows.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
        rows.truncate(8);
        rows
    }
}
