//! Prose masking: byte ranges (code fences, inline code, URLs) that lint and
//! fix must not touch, so tells in prose aren't confused with code.
// ponytail: no HTML tokenizer yet, add one when an HTML consumer shows up.
// ponytail: 4-space indented code blocks are NOT masked. The naive rule (any
// 4-space indent) also swallows nested list prose, which would silently stop
// linting it. Needs real block parsing, do that when a corpus actually uses
// indented code instead of fences.

use regex::Regex;
use std::sync::OnceLock;

fn re_inline() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`[^`\n]+`").unwrap())
}

fn re_url() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"https?://[^\s)>"'\]]+"#).unwrap())
}

/// Sorted, merged masked byte ranges.
pub fn mask(text: &str) -> Vec<(usize, usize)> {
    let mut m: Vec<(usize, usize)> = Vec::new();

    let mut fence_start: Option<usize> = None;
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            match fence_start.take() {
                None => fence_start = Some(pos),
                Some(s) => m.push((s, pos + line.len())),
            }
        }
        pos += line.len();
    }
    if let Some(s) = fence_start {
        m.push((s, text.len()));
    }

    for re in [re_inline(), re_url()] {
        for mm in re.find_iter(text) {
            if !covered(&m, mm.start()) {
                m.push((mm.start(), mm.end()));
            }
        }
    }

    m.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in m {
        match merged.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => merged.push((s, e)),
        }
    }
    merged
}

fn covered(mask: &[(usize, usize)], pos: usize) -> bool {
    mask.iter().any(|&(s, e)| pos >= s && pos < e)
}

pub fn overlaps(mask: &[(usize, usize)], start: usize, end: usize) -> bool {
    mask.iter().any(|&(s, e)| start < e && end > s)
}

/// Complement of the mask: the prose segments, in order.
pub fn prose_segments(text: &str, mask: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut segs = Vec::new();
    let mut pos = 0;
    for &(s, e) in mask {
        if pos < s {
            segs.push((pos, s));
        }
        pos = e;
    }
    if pos < text.len() {
        segs.push((pos, text.len()));
    }
    segs
}
