//! fix: deterministic, reviewable rewrites keyed to rules. Diff in, diff out.
//! Never touches masked regions (code, URLs) and never rewrites structure.

use crate::mask::{mask, prose_segments};
use crate::Engine;
use regex::{Captures, Regex};
use std::sync::OnceLock;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        fn $name() -> &'static Regex {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new($pat).unwrap())
        }
    };
}

// Numeric ranges FIRST: a dash between numbers becomes "to", never a comma.
// Regression: the TS deAi pass turned "2007—2024" into "2007, 2024" in prod.
re!(re_num_range, r"(\d)\s*[\u{2013}\u{2014}]\s*(\d)");
re!(re_em_dash, r"\s*\u{2014}\s*");
re!(re_sp_en_dash, r" \u{2013} ");
re!(re_word, r"[A-Za-z]+");
re!(re_spaces, r"[ \t]{2,}");
re!(re_dbl_comma, r",\s*,");
re!(re_sp_punct, r"[ \t]+([,.;:!?])");
re!(re_punct_dot, r"([,;:])\s*\.");
re!(re_recap, r"([.!?]\s+)([a-z])");

impl Engine {
    pub fn fix(&self, text: &str) -> String {
        let masked = mask(text);
        let mut out = String::with_capacity(text.len());
        let mut pos = 0;
        for (s, e) in prose_segments(text, &masked) {
            out.push_str(&text[pos..s]); // masked run, verbatim
            out.push_str(&self.fix_prose(&text[s..e]));
            pos = e;
        }
        out.push_str(&text[pos..]);

        // If a leading opener was stripped, restore the sentence-initial capital.
        let orig_upper = text
            .chars()
            .find(|c| c.is_ascii_alphabetic())
            .is_some_and(|c| c.is_ascii_uppercase());
        if orig_upper {
            if let Some(i) = out.find(|c: char| c.is_ascii_alphabetic()) {
                let upper = out[i..i + 1].to_ascii_uppercase();
                out.replace_range(i..i + 1, &upper);
            }
        }
        out
    }

    fn fix_prose(&self, s: &str) -> String {
        let mut s = re_num_range().replace_all(s, "$1 to $2").into_owned();
        s = re_em_dash().replace_all(&s, ", ").into_owned();
        s = re_sp_en_dash().replace_all(&s, ", ").into_owned();

        // Pack-order phrase/regex replacements (quote normalization comes first
        // in the base pack so later phrase regexes see straight quotes).
        for (re, repl) in &self.fix_res {
            s = re.replace_all(&s, repl.as_str()).into_owned();
        }

        // Case-preserving whole-word swaps with the noun-context guard.
        // ponytail: preceding-word list, not POS tagging. Upgrade if the
        // exception lists stop scaling.
        let mut out = String::with_capacity(s.len());
        let mut last = 0;
        for m in re_word().find_iter(&s) {
            out.push_str(&s[last..m.start()]);
            let w = m.as_str();
            match self.swaps.get(&w.to_lowercase()) {
                Some((repl, ri)) if !crate::skip_by_preceding(&self.rules[*ri], &s, m.start()) => {
                    out.push_str(&preserve_case(w, repl))
                }
                _ => out.push_str(w),
            }
            last = m.end();
        }
        out.push_str(&s[last..]);

        // Cleanup artifacts, then recapitalize after sentence enders only.
        let mut s = collapse_runs(&out);
        s = re_dbl_comma().replace_all(&s, ",").into_owned();
        s = re_sp_punct().replace_all(&s, "$1").into_owned();
        s = re_punct_dot().replace_all(&s, ".").into_owned();
        re_recap()
            .replace_all(&s, |c: &Captures| {
                format!("{}{}", &c[1], c[2].to_uppercase())
            })
            .into_owned()
    }
}

/// Collapse space runs left behind by deletions, mid-line only. Leading indent
/// is markdown structure (nested lists, indented code) and two trailing spaces
/// are a hard line break, so both survive. The TS pass ate them, see c012aab.
fn collapse_runs(s: &str) -> String {
    re_spaces()
        .replace_all(s, |c: &Captures| {
            let m = c.get(0).unwrap();
            let line_start = s[..m.start()].chars().next_back().is_none_or(|c| c == '\n');
            let line_end = s[m.end()..].chars().next().is_none_or(|c| c == '\n');
            if line_start || line_end {
                m.as_str().to_string()
            } else {
                " ".to_string()
            }
        })
        .into_owned()
}

/// Preserve the ORIGINAL token's case shape (UPPER / Capitalized / lower).
fn preserve_case(src: &str, repl: &str) -> String {
    if src.len() > 1 && src.bytes().all(|b| b.is_ascii_uppercase()) {
        return repl.to_ascii_uppercase();
    }
    if src
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_uppercase())
    {
        let mut chars = repl.chars();
        return match chars.next() {
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        };
    }
    repl.to_string()
}
