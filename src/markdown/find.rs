//! Finding and replacing text in a page's source.

use std::ops::Range;

/// The non-overlapping matches of `query` in `text`.
pub fn find_all(text: &str, query: &str, match_case: bool) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let fold = |c: char| if match_case { c } else { c.to_lowercase().next().unwrap_or(c) };
    let q: Vec<char> = query.chars().map(fold).collect();
    let chars: Vec<(usize, char)> = text.char_indices().map(|(i, c)| (i, fold(c))).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + q.len() <= chars.len() {
        if chars[i..i + q.len()].iter().zip(&q).all(|((_, c), q)| c == q) {
            let start = chars[i].0;
            let end = chars.get(i + q.len()).map_or(text.len(), |(j, _)| *j);
            out.push(start..end);
            i += q.len();
        } else {
            i += 1;
        }
    }
    out
}

/// The text with every match replaced, and how many there were.
pub fn replace_all(text: &str, query: &str, with: &str, match_case: bool) -> (String, usize) {
    let matches = find_all(text, query, match_case);
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for m in &matches {
        out.push_str(&text[pos..m.start]);
        out.push_str(with);
        pos = m.end;
    }
    out.push_str(&text[pos..]);
    (out, matches.len())
}

/// The match after (or, going back, before) `at`, wrapping around.
pub fn next_match(matches: &[Range<usize>], at: usize, back: bool) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(if back {
        matches.iter().rposition(|m| m.start < at).unwrap_or(matches.len() - 1)
    } else {
        matches.iter().position(|m| m.start >= at).unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding() {
        assert_eq!(find_all("Foo foo FOO", "foo", false), vec![0..3, 4..7, 8..11]);
        assert_eq!(find_all("Foo foo FOO", "foo", true), vec![4..7]);
        assert_eq!(find_all("aaaa", "aa", true), vec![0..2, 2..4]);
        assert_eq!(find_all("日本語と日本", "日本", true), vec![0..6, 12..18]);
        assert!(find_all("x", "", false).is_empty());
    }

    #[test]
    fn replacing() {
        assert_eq!(replace_all("foo x Foo", "foo", "bar", false), ("bar x bar".to_string(), 2));
    }

    #[test]
    fn next_and_previous() {
        let m = vec![0..1, 5..6, 9..10];
        assert_eq!(next_match(&m, 3, false), Some(1));
        assert_eq!(next_match(&m, 10, false), Some(0));
        assert_eq!(next_match(&m, 5, true), Some(0));
        assert_eq!(next_match(&m, 0, true), Some(2));
        assert_eq!(next_match(&[], 0, true), None);
    }
}
