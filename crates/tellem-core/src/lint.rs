//! lint: find AI tells, cite each one. Bands, never a human-vs-AI verdict.

use crate::mask::{mask, overlaps, prose_segments};
use crate::{skip_by_preceding, Band, Engine, Finding, Report};
use regex::Regex;
use std::sync::OnceLock;

fn re_sentence() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[.!?]+(?:\s+|$)").unwrap())
}

fn re_triad() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b[A-Za-z]+, [A-Za-z]+, and [A-Za-z]+\b").unwrap())
}

impl Engine {
    pub fn lint(&self, text: &str) -> Report {
        let masked = mask(text);
        let mut findings: Vec<Finding> = Vec::new();

        if let Some(ac) = &self.ac {
            for m in ac.find_iter(text) {
                let (start, end) = (m.start(), m.end());
                if overlaps(&masked, start, end) || !bounded(text.as_bytes(), start, end) {
                    continue;
                }
                let rule = &self.rules[self.ac_rule[m.pattern().as_usize()]];
                if skip_by_preceding(rule, text, start) {
                    continue;
                }
                findings.push(self.finding(rule, text, start, end));
            }
        }

        for (ri, re) in &self.regex_rules {
            let rule = &self.rules[*ri];
            for m in re.find_iter(text) {
                if overlaps(&masked, m.start(), m.end()) || skip_by_preceding(rule, text, m.start())
                {
                    continue;
                }
                findings.push(self.finding(rule, text, m.start(), m.end()));
            }
        }

        self.structural(text, &masked, &mut findings);
        findings.sort_by_key(|f| (f.start, f.end));

        let words: usize = prose_segments(text, &masked)
            .iter()
            .map(|&(s, e)| text[s..e].split_whitespace().count())
            .sum();
        // + 0.0 because f32's empty sum is -0.0, which prints as "-0.0" in JSON
        let total_weight: f32 = findings.iter().map(|f| f.weight).sum::<f32>() + 0.0;
        let kw = words as f32 / 1000.0;
        let score = if words == 0 { 0.0 } else { total_weight / kw };
        let band = if score >= self.bands.heavy {
            Band::Heavy
        } else if score >= self.bands.seasoned {
            Band::Seasoned
        } else {
            Band::Clean
        };
        Report {
            findings,
            words,
            total_weight,
            score,
            band,
        }
    }

    fn finding(&self, rule: &crate::Rule, text: &str, start: usize, end: usize) -> Finding {
        Finding {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            start,
            end,
            excerpt: excerpt(text, start, end),
            weight: rule.weight,
            rationale: rule.rationale.clone(),
            fixable: !rule.swaps.is_empty() || rule.replace.is_some(),
        }
    }

    /// S-rules live in code: they need document-level stats, not patterns.
    /// fix never touches them, they surface via --suggest (SPEC component 2).
    fn structural(&self, text: &str, masked: &[(usize, usize)], findings: &mut Vec<Finding>) {
        // S001 uniform bullet openers
        let mut bullets: Vec<(usize, String)> = Vec::new();
        let mut pos = 0;
        for line in text.split_inclusive('\n') {
            let t = line.trim_start();
            let is_bullet = t.starts_with("- ")
                || t.starts_with("* ")
                || t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains(". ");
            if is_bullet && !overlaps(masked, pos, pos + line.len()) {
                let body = t.trim_start_matches(['-', '*', ' ']);
                let body = body.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
                if let Some(w) = body.split_whitespace().next() {
                    bullets.push((pos, w.trim_start_matches(['*', '_']).to_lowercase()));
                }
            }
            pos += line.len();
        }
        if bullets.len() >= 4 {
            let mut counts = std::collections::HashMap::new();
            for (_, w) in &bullets {
                *counts.entry(w.clone()).or_insert(0usize) += 1;
            }
            if let Some((word, n)) = counts.into_iter().max_by_key(|&(_, n)| n) {
                if n as f32 / bullets.len() as f32 >= 0.8 {
                    let at = bullets[0].0;
                    findings.push(Finding {
                        rule_id: "S001".into(),
                        rule_name: "uniform-bullet-openers".into(),
                        start: at,
                        end: at,
                        excerpt: format!("{n}/{} bullets open with \"{word}\"", bullets.len()),
                        weight: 2.0,
                        rationale: "humans vary list openers, machines repeat them".into(),
                        fixable: false,
                    });
                }
            }
        }

        // S002 uniform sentence length (low coefficient of variation)
        let lens: Vec<f32> = re_sentence()
            .split(text)
            .map(|s| s.split_whitespace().count() as f32)
            .filter(|&n| n >= 3.0)
            .collect();
        if lens.len() >= 8 {
            let mean = lens.iter().sum::<f32>() / lens.len() as f32;
            let var = lens.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / lens.len() as f32;
            let cv = var.sqrt() / mean;
            if cv < 0.3 {
                findings.push(Finding {
                    rule_id: "S002".into(),
                    rule_name: "uniform-sentence-length".into(),
                    start: 0,
                    end: 0,
                    excerpt: format!("{} sentences, length CV {cv:.2}", lens.len()),
                    weight: 2.0,
                    rationale: "human sentence length is bursty, low variance reads machine".into(),
                    fixable: false,
                });
            }
        }

        // S003 triadic-list overuse, only when clustered
        let triads: Vec<_> = re_triad()
            .find_iter(text)
            .filter(|m| !overlaps(masked, m.start(), m.end()))
            .collect();
        if triads.len() >= 3 {
            for m in triads {
                findings.push(Finding {
                    rule_id: "S003".into(),
                    rule_name: "triadic-overuse".into(),
                    start: m.start(),
                    end: m.end(),
                    excerpt: excerpt(text, m.start(), m.end()),
                    weight: 0.25,
                    rationale: "X, Y, and Z cadence at high density is a machine rhythm".into(),
                    fixable: false,
                });
            }
        }
    }
}

fn bounded(text: &[u8], start: usize, end: usize) -> bool {
    let before_ok = start == 0
        || !text[start].is_ascii_alphanumeric()
        || !text[start - 1].is_ascii_alphanumeric();
    let after_ok = end >= text.len()
        || !text[end - 1].is_ascii_alphanumeric()
        || !text[end].is_ascii_alphanumeric();
    before_ok && after_ok
}

fn excerpt(text: &str, start: usize, end: usize) -> String {
    let slice = text.get(start..end).unwrap_or("");
    if slice.chars().count() <= 60 {
        slice.to_string()
    } else {
        let cut: String = slice.chars().take(57).collect();
        format!("{cut}...")
    }
}
