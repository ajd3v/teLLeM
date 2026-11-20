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

fn re_aria_hidden() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)\saria-hidden\s*=\s*(?:"true"|'true'|true(?:\s|/?>))"#).unwrap()
    })
}

/// Decorative HTML examples are mentions, like code blocks. Track matching
/// tags so a nested div cannot end a hidden region before its outer div.
fn decorative_html(text: &str, masked: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut hidden: Option<(usize, String, usize)> = None;
    for m in re_html_tag().find_iter(text) {
        if covered(masked, m.start()) {
            continue;
        }
        let tag = m.as_str();
        let closing = tag.starts_with("</");
        let name = tag
            .trim_start_matches(['<', '/'])
            .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .next()
            .unwrap_or("");
        let self_closing = tag.ends_with("/>")
            || matches!(
                name.to_ascii_lowercase().as_str(),
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            );
        if let Some((start, hidden_name, depth)) = &mut hidden {
            if name.eq_ignore_ascii_case(hidden_name) {
                if closing {
                    *depth -= 1;
                    if *depth == 0 {
                        ranges.push((*start, m.end()));
                        hidden = None;
                    }
                } else if !self_closing {
                    *depth += 1;
                }
            }
        } else if !closing && !self_closing && re_aria_hidden().is_match(tag) {
            hidden = Some((m.start(), name.to_string(), 1));
        }
    }
    ranges
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

    // Blocks first, then decorative regions, before masking individual tags.
    m = add_matches(text, m, re_html_block());
    m.extend(decorative_html(text, &m));
    m = merge(m);
    for re in [re_html_tag(), re_inline(), re_url()] {
        m = add_matches(text, m, re);
    }
    m
}

fn add_matches(text: &str, mut masked: Vec<(usize, usize)>, re: &Regex) -> Vec<(usize, usize)> {
    let added: Vec<_> = re
        .find_iter(text)
        .filter(|m| !covered(&masked, m.start()))
        .map(|m| (m.start(), m.end()))
        .collect();
    masked.extend(added);
    merge(masked)
}

pub(crate) fn merge(mut m: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
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
    let i = mask.partition_point(|&(_, e)| e <= pos);
    mask.get(i).is_some_and(|&(s, _)| s <= pos)
}

pub fn overlaps(mask: &[(usize, usize)], start: usize, end: usize) -> bool {
    let i = mask.partition_point(|&(_, e)| e <= start);
    mask.get(i).is_some_and(|&(s, _)| end > s)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_masks_preserve_pass_precedence() {
        let text = "```\n`code` https://fence.test\n```\n<script>`html` <b>x</b></script> `inline` https://url.test";
        let ranges = mask(text);
        let slices: Vec<_> = ranges.iter().map(|&(s, e)| &text[s..e]).collect();
        assert_eq!(
            slices,
            [
                "```\n`code` https://fence.test\n```\n<script>`html` <b>x</b></script>",
                "`inline`",
                "https://url.test"
            ]
        );
    }

    #[test]
    fn range_queries_match_linear_scan_at_boundaries() {
        let ranges = [(2, 5), (8, 12), (15, 20)];
        for start in 0..=22 {
            assert_eq!(
                covered(&ranges, start),
                ranges.iter().any(|&(s, e)| s <= start && start < e)
            );
            for end in start..=22 {
                assert_eq!(
                    overlaps(&ranges, start, end),
                    ranges.iter().any(|&(s, e)| start < e && end > s)
                );
            }
        }
    }
}
