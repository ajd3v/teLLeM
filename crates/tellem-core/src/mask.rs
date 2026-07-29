//! Prose masking: byte ranges (code fences, inline code, URLs, HTML markup)
//! that lint and fix must not touch, so tells in prose aren't confused with
//! code. Nothing here parses a language, it finds the regions to leave alone.
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

/// HTML tags, and whole script/style/svg/code bodies. Attribute text is markup
/// rather than prose: a class name full of words would otherwise read as a
/// sentence. `<code>` is masked for the same reason markdown backticks are,
/// since a page that names `delve` as a rule is mentioning it, not using it.
// ponytail: regex, not a tokenizer. It masks a `<` inside a string in JS as if
// it opened a tag, which over-masks and can only ever cause a missed finding,
// never a false one. Reach for a real parser when that starts costing findings.
fn re_html_block() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Written out per tag because the regex crate has no backreferences.
    RE.get_or_init(|| {
        Regex::new(
            r"(?is)<script\b[^>]*>.*?</script\s*>|<style\b[^>]*>.*?</style\s*>|<svg\b[^>]*>.*?</svg\s*>|<code\b[^>]*>.*?</code\s*>|<!--.*?-->",
        )
        .unwrap()
    })
}

fn re_html_tag() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)<[a-zA-Z/!][^>]*>").unwrap())
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

    // Blocks first: a <script> body is masked whole, so tags inside it are
    // already covered and the tag pass skips them.
    for re in [re_html_block(), re_html_tag(), re_inline(), re_url()] {
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
