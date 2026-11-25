//! Address boundaries and the soft breaks used by the page view.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Visible URLs, including those in a link label or in revealed Markdown.
pub(super) fn ranges(text: &str) -> Vec<Range<usize>> {
    if !text.contains([':', '.']) {
        return Vec::new();
    }
    let scheme_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-');
    let mut ranges = Vec::<Range<usize>>::new();
    for (start, ch) in text.char_indices() {
        if !ch.is_ascii_alphabetic()
            || ranges.last().is_some_and(|r| r.end > start)
            || text[..start].chars().next_back().is_some_and(scheme_char)
        {
            continue;
        }
        let rest = &text[start..];
        let scheme_len = rest.chars().take_while(|&c| scheme_char(c)).count();
        let (scheme, after) = rest.split_at(scheme_len);
        let prefix = if rest.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("www.")) {
            4
        } else if after.starts_with("://") {
            scheme_len + 3
        } else if after.starts_with(':') && ["mailto", "file", "data"].iter().any(|s| scheme.eq_ignore_ascii_case(s)) {
            scheme_len + 1
        } else {
            continue;
        };
        let (mut parens, mut brackets) = (0, 0);
        let len = rest[prefix..]
            .find(|c: char| match c {
                '(' => {
                    parens += 1;
                    false
                }
                '[' => {
                    brackets += 1;
                    false
                }
                ')' if parens > 0 => {
                    parens -= 1;
                    false
                }
                ']' if brackets > 0 => {
                    brackets -= 1;
                    false
                }
                _ => c.is_whitespace() || matches!(c, ')' | ']' | '<' | '>' | '"' | '`' | '、' | '。' | '\u{fffc}'),
            })
            .map_or(rest.len(), |i| prefix + i);
        let end = rest[..len].trim_end_matches(['.', ',', ':', ';', '!', '?', '\'']).len();
        if end > prefix {
            ranges.push(start..start + end);
        }
    }
    ranges
}

pub(super) fn breaks(text: &str) -> Vec<usize> {
    ranges(text).into_iter().flat_map(|r| character_breaks(&text[r.clone()]).map(move |i| r.start + i)).collect()
}

pub(super) fn character_breaks(text: &str) -> impl Iterator<Item = usize> {
    text.grapheme_indices(true).skip(1).map(|(i, _)| i)
}

const SOFT_BREAK: char = '\u{200b}';

/// Supply Unicode line-break opportunities to iced without changing the
/// displayed characters. A break at a span boundary belongs to the left span.
pub(super) fn layout_text(text: &str, range: Range<usize>, breaks: &[usize]) -> String {
    let first = breaks.partition_point(|&i| i <= range.start);
    let end = breaks.partition_point(|&i| i <= range.end);
    let mut output = String::with_capacity(range.len() + (end - first) * SOFT_BREAK.len_utf8());
    let mut start = range.start;
    for &at in &breaks[first..end] {
        output.push_str(&text[start..at]);
        output.push(SOFT_BREAK);
        start = at;
    }
    output.push_str(&text[start..range.end]);
    output
}

/// Map a shaped offset back to the original bytes, including hits on a break.
pub(super) fn text_offset(offset: usize, breaks: &[usize]) -> usize {
    let (mut left, mut right) = (0, breaks.len());
    while left < right {
        let mid = left + (right - left) / 2;
        if layout_len(breaks[mid], mid) < offset {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    let index = offset.saturating_sub(left * SOFT_BREAK.len_utf8());
    if left == 0 { index } else { index.max(breaks[left - 1]) }
}

pub(super) fn layout_len(len: usize, breaks: usize) -> usize {
    len + breaks * SOFT_BREAK.len_utf8()
}
