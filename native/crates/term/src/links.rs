//! plain http(s) URLs in the text, found as xterm.js's web-links addon finds them:
//! `(https?|HTTPS?)://[^\s"'!*(){}|\\^<>`]*[^\s"':,.!?{}|\\^~\[\]`()<>]`

use std::ops::Range;

fn inside(c: char) -> bool {
    !(c.is_whitespace() || c.is_control() || "\"'!*(){}|\\^<>`".contains(c))
}
fn last(c: char) -> bool {
    !(c.is_whitespace() || c.is_control() || "\"':,.!?{}|\\^~[]`()<>".contains(c))
}

/// the URLs in a line, as ranges of char indices
pub fn urls(chars: &[char]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let Some(after) = scheme(&chars[i..]) else {
            i += 1;
            continue;
        };
        let start = i;
        let mut end = start + after;
        while end < chars.len() && inside(chars[end]) {
            end += 1;
        }
        while end > start + after && !last(chars[end - 1]) {
            end -= 1;
        }
        if end > start + after {
            out.push(start..end);
            i = end;
        } else {
            i = start + after;
        }
    }
    out
}

// the length of "http://" or "https://" at the start (both all lower or all upper case)
fn scheme(c: &[char]) -> Option<usize> {
    for s in ["https://", "http://", "HTTPS://", "HTTP://"] {
        if c.len() >= s.len() && s.chars().zip(c).all(|(a, b)| a == *b) {
            return Some(s.len());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(s: &str) -> Vec<String> {
        let c: Vec<char> = s.chars().collect();
        urls(&c).into_iter().map(|r| c[r].iter().collect()).collect()
    }

    #[test]
    fn finds_urls() {
        assert_eq!(found("see https://claude.ai/code/session_01 now"), ["https://claude.ai/code/session_01"]);
        assert_eq!(found("(http://x.org/a?b=1&c=2#d)."), ["http://x.org/a?b=1&c=2#d"]);
        assert_eq!(found("at https://x.org/a, then"), ["https://x.org/a"]);
        assert_eq!(found("'https://x.org/~me/'"), ["https://x.org/~me/"]);
        assert_eq!(found("a https://a.b c HTTPS://C.D"), ["https://a.b", "HTTPS://C.D"]);
        assert_eq!(found("╭ https://é.example/ü │"), ["https://é.example/ü"]);
        assert!(found("ftp://x.org https:// file:///tmp").is_empty());
    }
}
