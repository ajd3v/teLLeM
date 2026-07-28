//! tellem-core: rules, scoring, and deterministic fixes for AI-text forensics.
//! Every finding carries its receipt (rule id + rationale). No human-vs-AI verdict.

pub mod attribute;
mod fix;
mod lint;
mod mask;
pub mod mine;

use std::collections::HashMap;

use aho_corasick::{AhoCorasick, MatchKind};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// The public base rule pack, embedded so the CLI works with zero setup.
// ponytail: include_str reaches outside the crate dir, breaks `cargo publish`.
// Move packs/ into the crate (or a build script) when publishing to crates.io.
pub const BASE_PACK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packs/base.toml"
));

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Word,
    Phrase,
    Regex,
}

#[derive(Deserialize, Clone)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: f32,
    pub rationale: String,
    /// word kind: matched form -> replacement (inflections listed explicitly)
    #[serde(default)]
    pub swaps: std::collections::BTreeMap<String, String>,
    /// phrase/regex kinds: replacement for fix ("" = delete). Absent = report-only.
    #[serde(default)]
    pub replace: Option<String>,
    /// skip match (lint and fix) when the preceding word is one of these
    #[serde(default)]
    pub unless_preceded_by: Vec<String>,
}

fn default_weight() -> f32 {
    1.0
}

#[derive(Deserialize, Clone, Copy)]
pub struct Bands {
    pub seasoned: f32,
    pub heavy: f32,
}

impl Default for Bands {
    fn default() -> Self {
        Bands {
            seasoned: 2.0,
            heavy: 6.0,
        }
    }
}

#[derive(Deserialize)]
pub struct Pack {
    pub meta: Meta,
    #[serde(default)]
    pub bands: Option<Bands>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Deserialize)]
pub struct Meta {
    pub name: String,
    pub version: String,
}

impl Pack {
    pub fn parse(toml_src: &str) -> Result<Pack, Error> {
        Ok(toml::from_str(toml_src)?)
    }
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Clean,
    Seasoned,
    Heavy,
}

impl std::fmt::Display for Band {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Band::Clean => "clean",
            Band::Seasoned => "seasoned",
            Band::Heavy => "heavy",
        })
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Finding {
    pub rule_id: String,
    pub rule_name: String,
    pub start: usize,
    pub end: usize,
    pub excerpt: String,
    pub weight: f32,
    pub rationale: String,
    /// true when `fix` deterministically removes this finding
    pub fixable: bool,
}

#[derive(Serialize, Debug)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub words: usize,
    /// sum of finding weights (the monotonic-guarantee metric)
    pub total_weight: f32,
    /// weighted tells per kiloword (the density metric behind bands)
    pub score: f32,
    pub band: Band,
}

pub struct Engine {
    pub(crate) rules: Vec<Rule>,
    pub(crate) ac: Option<AhoCorasick>,
    /// AC pattern index -> rules index
    pub(crate) ac_rule: Vec<usize>,
    /// lint-time regex rules (rules index, compiled)
    pub(crate) regex_rules: Vec<(usize, Regex)>,
    /// fix-time replacements in pack order (compiled, replacement)
    pub(crate) fix_res: Vec<(Regex, String)>,
    /// word form -> (replacement, rules index) for fix
    pub(crate) swaps: HashMap<String, (String, usize)>,
    pub(crate) bands: Bands,
}

impl Engine {
    /// Later packs override earlier rules by id, or append new ones.
    pub fn from_packs(packs: &[Pack]) -> Result<Engine, Error> {
        let mut rules: Vec<Rule> = Vec::new();
        let mut by_id: HashMap<String, usize> = HashMap::new();
        let mut bands = Bands::default();
        for p in packs {
            if let Some(b) = p.bands {
                bands = b;
            }
            for r in &p.rules {
                match by_id.get(&r.id) {
                    Some(&i) => rules[i] = r.clone(),
                    None => {
                        by_id.insert(r.id.clone(), rules.len());
                        rules.push(r.clone());
                    }
                }
            }
        }

        let mut ac_pats: Vec<String> = Vec::new();
        let mut ac_rule: Vec<usize> = Vec::new();
        let mut regex_rules = Vec::new();
        let mut fix_res = Vec::new();
        let mut swaps = HashMap::new();

        for (i, r) in rules.iter().enumerate() {
            match r.kind {
                Kind::Word => {
                    for (form, repl) in &r.swaps {
                        ac_pats.push(form.to_lowercase());
                        ac_rule.push(i);
                        swaps.insert(form.to_lowercase(), (repl.clone(), i));
                    }
                    for p in &r.patterns {
                        ac_pats.push(p.to_lowercase());
                        ac_rule.push(i);
                    }
                }
                Kind::Phrase => {
                    for p in &r.patterns {
                        ac_pats.push(p.to_lowercase());
                        ac_rule.push(i);
                    }
                    if let Some(repl) = &r.replace {
                        let alts: Vec<String> =
                            r.patterns.iter().map(|p| regex::escape(p)).collect();
                        let re = Regex::new(&format!(r"(?i)\b(?:{})\b", alts.join("|")))?;
                        fix_res.push((re, repl.clone()));
                    }
                }
                Kind::Regex => {
                    let pat = r
                        .pattern
                        .as_deref()
                        .ok_or_else(|| format!("rule {}: regex kind needs `pattern`", r.id))?;
                    regex_rules.push((i, Regex::new(pat)?));
                    if let Some(repl) = &r.replace {
                        fix_res.push((Regex::new(pat)?, repl.clone()));
                    }
                }
            }
        }

        let ac = if ac_pats.is_empty() {
            None
        } else {
            Some(
                AhoCorasick::builder()
                    .ascii_case_insensitive(true)
                    .match_kind(MatchKind::LeftmostLongest)
                    .build(&ac_pats)?,
            )
        };

        Ok(Engine {
            rules,
            ac,
            ac_rule,
            regex_rules,
            fix_res,
            swaps,
            bands,
        })
    }

    /// Rule ids whose findings `fix` claims to remove (the completeness contract).
    pub fn fixable_rule_ids(&self) -> Vec<&str> {
        self.rules
            .iter()
            .filter(|r| !r.swaps.is_empty() || r.replace.is_some())
            .map(|r| r.id.as_str())
            .collect()
    }
}

/// The word immediately before byte `start`, lowercased ("" at text start).
pub(crate) fn preceding_word(text: &str, start: usize) -> String {
    let head = &text.as_bytes()[..start];
    let mut end = head.len();
    while end > 0 && !head[end - 1].is_ascii_alphanumeric() {
        end -= 1;
    }
    let mut begin = end;
    while begin > 0 && head[begin - 1].is_ascii_alphabetic() {
        begin -= 1;
    }
    text[begin..end].to_lowercase()
}

pub(crate) fn skip_by_preceding(rule: &Rule, text: &str, start: usize) -> bool {
    !rule.unless_preceded_by.is_empty()
        && rule
            .unless_preceded_by
            .contains(&preceding_word(text, start))
}
