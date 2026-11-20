//! Bounded sentence shapes selected by a pack, with source byte spans.
use crate::mask::{merge, prose_segments};
use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    RelabelPairing,
    OrdinalDrumbeat,
    NounChiasmus,
}

struct Sentence<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

impl Shape {
    pub(crate) fn find(self, text: &str, masked: &[(usize, usize)]) -> Vec<(usize, usize)> {
        let mut findings = Vec::new();
        for (start, end) in prose_segments(text, &barriers(text, masked)) {
            let sentences = sentences(&text[start..end], start);
            if matches!(self, Self::NounChiasmus) {
                for sentence in &sentences {
                    if let Some((start, end)) = noun_chiasmus(sentence.text) {
                        findings.push((sentence.start + start, sentence.start + end));
                    }
                }
                continue;
            }
            for pair in sentences.windows(2) {
                // The source describes short clauses in a 220-character window.
                // Eighteen words retains its 14-word positive and rejects its
                // 19-word negative. Neither threshold is an authorship claim.
                if text[pair[0].start..pair[1].end].chars().count() > 220
                    || pair.iter().any(|s| s.text.split_whitespace().count() > 18)
                {
                    continue;
                }
                let found = match self {
                    Self::RelabelPairing => pair.iter().all(|s| relabel(s.text)),
                    Self::OrdinalDrumbeat => ordinal_pair(pair[0].text, pair[1].text),
                    Self::NounChiasmus => unreachable!(),
                };
                if found {
                    findings.push((pair[0].start, pair[1].end));
                }
            }
        }
        findings
    }
}

fn barriers(text: &str, masked: &[(usize, usize)]) -> Vec<(usize, usize)> {
    static PREFIX: OnceLock<Regex> = OnceLock::new();
    let prefix = PREFIX.get_or_init(|| {
        Regex::new(r"^[ \t]*(?:#{1,6}[ \t]+|(?:[-+*]|[0-9]+[.)])[ \t]+|>[ \t]*)").unwrap()
    });
    let mut barriers = masked.to_vec();
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        let setext = !trimmed.is_empty()
            && (trimmed.chars().all(|c| c == '=') || trimmed.chars().all(|c| c == '-'));
        if trimmed.is_empty() || setext {
            barriers.push((pos, pos + line.len()));
        } else if let Some(m) = prefix.find(line) {
            barriers.push((pos, pos + m.end()));
            if line.trim_start().starts_with('#') && line.ends_with('\n') {
                barriers.push((pos + line.len() - 1, pos + line.len()));
            }
        }
        for (i, _) in line.match_indices('|') {
            if i == 0 || !line[..i].ends_with('\\') {
                barriers.push((pos + i, pos + i + 1));
            }
        }
        pos += line.len();
    }
    merge(barriers)
}

fn sentences(text: &str, offset: usize) -> Vec<Sentence<'_>> {
    text.split_inclusive(['.', '!', '?'])
        .scan(offset, |pos, part| {
            let start = *pos + part.len() - part.trim_start().len();
            *pos += part.len();
            let trimmed = part.trim();
            Some(Sentence {
                text: trimmed,
                start,
                end: start + trimmed.len(),
            })
        })
        .filter(|sentence| !sentence.text.is_empty())
        .collect()
}

fn relabel(text: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)\bcalls?\s+(?:it|this|that|them|him|her)\s+([a-z]+)\b").unwrap()
    });
    let Some(c) = re.captures(text) else {
        return false;
    };
    let subject_words = text[..c.get(0).unwrap().start()].split_whitespace().count();
    if !(1..=6).contains(&subject_words) {
        return false;
    }
    !matches!(
        c[1].to_ascii_lowercase().as_str(),
        "once"
            | "twice"
            | "again"
            | "often"
            | "always"
            | "never"
            | "sometimes"
            | "regularly"
            | "repeatedly"
            | "directly"
            | "indirectly"
            | "asynchronously"
            | "synchronously"
            | "recursively"
            | "immediately"
            | "later"
            | "now"
            | "here"
            | "automatically"
            | "manually"
            | "periodically"
            | "frequently"
            | "rarely"
            | "usually"
            | "normally"
            | "conditionally"
            | "unconditionally"
            | "explicitly"
            | "implicitly"
            | "locally"
            | "remotely"
            | "simultaneously"
            | "concurrently"
            | "sequentially"
            | "there"
            | "first"
            | "last"
            | "every"
            | "each"
            | "when"
            | "before"
            | "after"
            | "from"
            | "with"
            | "without"
            | "on"
            | "at"
            | "in"
            | "to"
            | "for"
            | "by"
    )
}

fn ordinal_pair(first: &str, second: &str) -> bool {
    static COUNT: OnceLock<Regex> = OnceLock::new();
    static ORDINAL: OnceLock<Regex> = OnceLock::new();
    let count = COUNT.get_or_init(|| {
        Regex::new(r"(?i)^(?:[0-9]+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+[a-z]+\b").unwrap()
    });
    // A finite verb list is conservative about the dropped noun. Accepting
    // any word here would also accept ordinary descriptions of a fourth test.
    let ordinal = ORDINAL.get_or_init(|| {
        Regex::new(r"(?i)^the (?:first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth|[0-9]+(?:st|nd|rd|th))\s+(?:asks|answers|says|does|is|was|has|can|will|would|should|must|shows|checks|ships|covers|fails|works|reads|writes|runs|takes|makes|gets|keeps|stays|looks|proves|holds|brings|cuts|builds|costs|means|needs|adds|removes|lets)\b").unwrap()
    });
    count.is_match(first) && ordinal.is_match(second)
}

fn noun_chiasmus(text: &str) -> Option<(usize, usize)> {
    static WORD: OnceLock<Regex> = OnceLock::new();
    let word = WORD.get_or_init(|| Regex::new(r"[A-Za-z]+(?:['\x{2019}][A-Za-z]+)?").unwrap());
    let words: Vec<_> = word.find_iter(text).collect();
    for (right, token) in words.iter().enumerate().skip(1) {
        if !indefinite(token.as_str())
            || !matches!(
                words[right - 1].as_str().to_ascii_lowercase().as_str(),
                "is" | "are" | "was" | "were"
            )
            || !text[words[right - 1].end()..token.start()]
                .chars()
                .all(char::is_whitespace)
        {
            continue;
        }
        let Some(right_head) = head_noun(text, &words, right) else {
            continue;
        };
        for left in (0..right - 1).rev() {
            if text[words[left].start()..words[right_head].end()]
                .chars()
                .count()
                > 220
            {
                break;
            }
            if !indefinite(words[left].as_str()) {
                continue;
            }
            let Some(left_head) = head_noun(text, &words, left) else {
                continue;
            };
            if left_head < right - 1
                && words[left_head]
                    .as_str()
                    .eq_ignore_ascii_case(words[right_head].as_str())
            {
                return Some((words[left].start(), words[right_head].end()));
            }
        }
    }
    None
}

fn indefinite(word: &str) -> bool {
    word.eq_ignore_ascii_case("a") || word.eq_ignore_ascii_case("an")
}

// Take the head of a short noun phrase, allowing up to two modifiers. Stop at
// relative clauses and prepositions rather than treating a shared adjective
// as the noun in two different phrases, such as a blue car and a blue truck.
fn head_noun(text: &str, words: &[regex::Match<'_>], determiner: usize) -> Option<usize> {
    let mut head = None;
    for i in determiner + 1..words.len() {
        let word = words[i].as_str().to_ascii_lowercase();
        if !text[words[i - 1].end()..words[i].start()]
            .chars()
            .all(char::is_whitespace)
            || matches!(
                word.as_str(),
                "a" | "an"
                    | "the"
                    | "that"
                    | "which"
                    | "who"
                    | "whom"
                    | "before"
                    | "after"
                    | "because"
                    | "when"
                    | "where"
                    | "while"
                    | "if"
                    | "is"
                    | "are"
                    | "was"
                    | "were"
                    | "has"
                    | "have"
                    | "had"
                    | "can"
                    | "will"
                    | "would"
                    | "should"
                    | "must"
                    | "it"
                    | "they"
                    | "we"
                    | "you"
                    | "he"
                    | "she"
                    | "i"
                    | "of"
                    | "in"
                    | "on"
                    | "at"
                    | "for"
                    | "to"
                    | "with"
                    | "without"
                    | "and"
                    | "or"
                    | "but"
            )
        {
            break;
        }
        if i > determiner + 3 {
            return None;
        }
        head = Some(i);
    }
    head
}
