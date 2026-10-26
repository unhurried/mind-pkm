//! Formatting commands of the page editor: inline styles, links, headings,
//! quotes and lists, clearing formatting, auto-pairing, task boxes and the
//! selection helpers. Like the editing rules, they map the page text and a
//! selection to new text and selection.

use super::edit::{Edited, MarkerKind, Sel, line_marker, line_range, splice};
use super::parse::{BlockKind, Document};
use pulldown_cmark::{Event, Tag, TagEnd};
use std::ops::Range;

/// An inline style that wraps text in delimiters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineStyle {
    Bold,
    Italic,
    Underline,
    Code,
    Strike,
    Highlight,
    Math,
    Subscript,
    Superscript,
}

impl InlineStyle {
    fn delimiters(self) -> (&'static str, &'static str) {
        match self {
            InlineStyle::Bold => ("**", "**"),
            InlineStyle::Italic => ("*", "*"),
            InlineStyle::Underline => ("<u>", "</u>"),
            InlineStyle::Code => ("`", "`"),
            InlineStyle::Strike => ("~~", "~~"),
            InlineStyle::Highlight => ("==", "=="),
            InlineStyle::Math => ("$", "$"),
            InlineStyle::Subscript => ("~", "~"),
            InlineStyle::Superscript => ("^", "^"),
        }
    }

    /// The character delimiters made of, when a run of it has a meaning by its length.
    fn run_char(self) -> Option<char> {
        match self {
            InlineStyle::Bold | InlineStyle::Italic => Some('*'),
            InlineStyle::Strike | InlineStyle::Subscript => Some('~'),
            _ => None,
        }
    }
}

/// How many `c` are right before `at` / right from `at`.
fn run_before(text: &str, at: usize, c: char) -> usize {
    text[..at].chars().rev().take_while(|&x| x == c).count()
}

fn run_after(text: &str, at: usize, c: char) -> usize {
    text[at..].chars().take_while(|&x| x == c).count()
}

/// Whether delimiters of `style` enclose `range` directly.
fn enclosed(text: &str, range: &Range<usize>, style: InlineStyle) -> bool {
    let (open, close) = style.delimiters();
    if range.start < open.len() || !text[..range.start].ends_with(open) || !text[range.end..].starts_with(close) {
        return false;
    }
    match (style, style.run_char()) {
        // `*` alone is italic, `**` bold, `***` both.
        (InlineStyle::Italic, Some(c)) => {
            let (b, a) = (run_before(text, range.start, c), run_after(text, range.end, c));
            b == a && b % 2 == 1
        }
        (InlineStyle::Bold, Some(c)) => {
            let (b, a) = (run_before(text, range.start, c), run_after(text, range.end, c));
            b == a && b >= 2
        }
        (InlineStyle::Subscript, Some(c)) => {
            run_before(text, range.start, c) == 1 && run_after(text, range.end, c) == 1
        }
        _ => true,
    }
}

/// The kind of character words are made of: a word is a run of one kind
/// (so that Japanese splits between kanji, hiragana and katakana).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WordClass {
    Han,
    Hiragana,
    Katakana,
    Other,
}

pub(crate) fn word_class(c: char) -> Option<WordClass> {
    match c {
        '\u{3040}'..='\u{309f}' => Some(WordClass::Hiragana),
        '\u{30a0}'..='\u{30ff}' | '\u{ff66}'..='\u{ff9f}' => Some(WordClass::Katakana),
        '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}' | '\u{f900}'..='\u{faff}' | '々' => Some(WordClass::Han),
        c if c.is_alphanumeric() || c == '_' => Some(WordClass::Other),
        _ => None,
    }
}

/// The word at `at`, if any: preferably the one holding `at`, else the one ending there.
pub fn word_at(text: &str, at: usize) -> Option<Range<usize>> {
    let class_at = |i: usize| text[i..].chars().next().and_then(word_class);
    let class = class_at(at).or_else(|| text[..at].chars().next_back().and_then(word_class))?;
    let mut start = at;
    while let Some(c) = text[..start].chars().next_back() {
        if word_class(c) != Some(class) {
            break;
        }
        start -= c.len_utf8();
    }
    let mut end = at;
    while let Some(c) = text[end..].chars().next() {
        if word_class(c) != Some(class) {
            break;
        }
        end += c.len_utf8();
    }
    (start < end).then_some(start..end)
}

/// Toggles an inline style on the selection, or the word at the cursor; with
/// neither, inserts the delimiters with the cursor between them.
pub fn toggle_inline(text: &str, sel: Sel, style: InlineStyle) -> Edited {
    let (open, close) = style.delimiters();
    let mut range = sel.range();
    if range.is_empty() {
        match word_at(text, range.start) {
            Some(w) => range = w,
            None => {
                let t = splice(text, range.clone(), &format!("{open}{close}"));
                let at = range.start + open.len();
                return Edited { text: t, sel: Sel::caret(at), pending: None };
            }
        }
    }
    let (s, e) = (range.start, range.end);
    if enclosed(text, &range, style) {
        let t = splice(&splice(text, e..e + close.len(), ""), s - open.len()..s, "");
        return Edited { text: t, sel: Sel { anchor: s - open.len(), head: e - open.len() }, pending: None };
    }
    let inner = &text[range.clone()];
    if inner.len() >= open.len() + close.len() && inner.starts_with(open) && inner.ends_with(close) {
        let unwrapped = &inner[open.len()..inner.len() - close.len()];
        let t = splice(text, range.clone(), unwrapped);
        return Edited { text: t, sel: Sel { anchor: s, head: s + unwrapped.len() }, pending: None };
    }
    let t = splice(text, range.clone(), &format!("{open}{inner}{close}"));
    Edited { text: t, sel: Sel { anchor: s + open.len(), head: e + open.len() }, pending: None }
}

/// A link around the selection, with the cursor in its (empty) URL; without
/// a selection, an empty link with the cursor in its text.
pub fn insert_link(text: &str, sel: Sel) -> Edited {
    let r = sel.range();
    let inner = &text[r.clone()];
    let t = splice(text, r.clone(), &format!("[{inner}]()"));
    let at = if inner.is_empty() { r.start + 1 } else { r.start + inner.len() + 3 };
    Edited { text: t, sel: Sel::caret(at), pending: None }
}

/// The heading level of a paragraph or heading block's source (0 for a paragraph), and its content range.
fn heading_parts(text: &str, region: &Range<usize>) -> Option<(u8, Range<usize>)> {
    let src = &text[region.clone()];
    let first = line_range(text, region.start);
    if let Some(m) = line_marker(&text[first.clone()])
        && let MarkerKind::Heading(n) = m.kind
    {
        // Without a closing sequence of `#`.
        let line = &text[first.start + m.end..first.end];
        let trimmed = line.trim_end();
        let without = trimmed.trim_end_matches('#');
        let end = if without.len() < trimmed.len() && (without.is_empty() || without.ends_with(' ')) {
            without.trim_end().len()
        } else {
            trimmed.len()
        };
        return Some((n, first.start + m.end..first.start + m.end + end));
    }
    // Setext: the text above an `===` / `---` underline.
    let lines: Vec<&str> = src.lines().collect();
    if lines.len() >= 2 {
        let last = lines[lines.len() - 1].trim();
        let underline = !last.is_empty() && (last.chars().all(|c| c == '=') || last.chars().all(|c| c == '-'));
        if underline {
            let level = if last.starts_with('=') { 1 } else { 2 };
            let content_end = region.start + src.len() - lines[lines.len() - 1].len();
            return Some((level, region.start..text[..content_end].trim_end().len()));
        }
    }
    Some((0, region.clone()))
}

/// Makes a paragraph or heading block a heading of `level` (0: a paragraph).
pub fn set_heading(text: &str, doc: &Document, region: Range<usize>, sel: Sel, level: u8) -> Option<Edited> {
    let kind = doc.blocks.iter().find(|b| b.range.start == region.start).map(|b| b.kind.clone());
    if !matches!(kind, None | Some(BlockKind::Paragraph | BlockKind::Heading(_))) {
        return None;
    }
    let (_, content) = heading_parts(text, &region)?;
    // A heading is one line.
    let body = text[content.clone()].split('\n').map(str::trim).collect::<Vec<_>>().join(" ");
    let marker = if level == 0 { String::new() } else { format!("{} ", "#".repeat(level.min(6) as usize)) };
    let new = format!("{marker}{body}");
    let t = splice(text, region.clone(), &new);
    let map = |o: usize| {
        let rel = o.saturating_sub(content.start).min(body.len());
        region.start + marker.len() + rel
    };
    Some(Edited { text: t, sel: Sel { anchor: map(sel.anchor), head: map(sel.head) }, pending: None })
}

/// The heading level of a paragraph or heading block (0 for a paragraph).
pub fn heading_level(text: &str, region: &Range<usize>) -> u8 {
    heading_parts(text, region).map_or(0, |(n, _)| n)
}

/// Ctrl+= (`raise`) / Ctrl+-: one heading level up or down; a paragraph
/// becomes a level-6 heading going up, a level-6 heading a paragraph going down.
pub fn step_heading(text: &str, doc: &Document, region: Range<usize>, sel: Sel, raise: bool) -> Option<Edited> {
    let level = heading_level(text, &region);
    let new = match (level, raise) {
        (0, true) => 6,
        (0, false) => return None,
        (n, true) => n.saturating_sub(1).max(1),
        (6, false) => 0,
        (n, false) => n + 1,
    };
    set_heading(text, doc, region, sel, new)
}

/// Quotes the lines of `range`, or unquotes them when they all are quoted.
pub fn toggle_quote(text: &str, range: Range<usize>, sel: Sel) -> Edited {
    let lines_range = line_range(text, range.start).start..line_range(text, range.end).end;
    let lines: Vec<&str> = text[lines_range.clone()].split('\n').collect();
    let quoted = lines.iter().filter(|l| !l.trim().is_empty()).all(|l| l.trim_start().starts_with('>'));
    let new: Vec<String> = lines
        .iter()
        .map(|l| {
            if quoted {
                let t = l.trim_start();
                t.strip_prefix("> ").or_else(|| t.strip_prefix('>')).unwrap_or(t).to_string()
            } else if l.trim().is_empty() {
                ">".to_string()
            } else {
                format!("> {l}")
            }
        })
        .collect();
    let new = new.join("\n");
    let delta = new.len() as isize - lines_range.len() as isize;
    let t = splice(text, lines_range.clone(), &new);
    let shift = |o: usize| -> usize {
        if quoted {
            o.saturating_sub(2).max(lines_range.start).min(lines_range.start + new.len())
        } else {
            (o as isize + if o == lines_range.start { 0 } else { 2 }) as usize
        }
    };
    let _ = delta;
    Edited { text: t, sel: Sel { anchor: shift(sel.anchor), head: shift(sel.head) }, pending: None }
}

/// Makes the blocks within `range` items of an ordered (or bullet) list, or
/// paragraphs when they all already are items of that kind of list.
pub fn toggle_list(text: &str, doc: &Document, range: Range<usize>, ordered: bool) -> Edited {
    let blocks: Vec<&super::parse::Block> = doc
        .blocks
        .iter()
        .filter(|b| b.range.end >= range.start && b.range.start <= range.end.max(range.start))
        .collect();
    if blocks.is_empty() {
        let marker = if ordered { "1. " } else { "- " };
        let t = splice(text, range.start..range.start, marker);
        return Edited { text: t, sel: Sel::caret(range.start + marker.len()), pending: None };
    }
    let span = blocks[0].range.start..blocks[blocks.len() - 1].range.end;
    let is_kind = |b: &&super::parse::Block| match &b.kind {
        BlockKind::ListItem(i) => i.number.is_some() == ordered,
        _ => false,
    };
    let all = blocks.iter().all(is_kind);
    let mut parts = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        let src = &text[b.range.clone()];
        let first_line = src.lines().next().unwrap_or("");
        let content = match line_marker(first_line) {
            Some(m) if !matches!(m.kind, MarkerKind::Quote) => &src[m.end.min(src.len())..],
            _ => src,
        };
        // Continuation lines without the old indentation.
        let lines: Vec<&str> = content.lines().map(|l| l.trim_start()).collect();
        if all {
            parts.push(lines.join("\n"));
        } else {
            let marker = if ordered { format!("{}. ", i + 1) } else { "- ".to_string() };
            let indent = " ".repeat(marker.len());
            parts.push(format!("{marker}{}", lines.join(&format!("\n{indent}"))));
        }
    }
    let new = parts.join(if all { "\n\n" } else { "\n" });
    let t = splice(text, span.clone(), &new);
    Edited { text: t, sel: Sel::caret(span.start + new.len()), pending: None }
}

/// Removes inline formatting (emphasis, strikethrough, highlight, code,
/// sub/superscript, inline HTML) from `range`, keeping text and links. The
/// range grows to whole formatted spans it touches.
pub fn clear_formatting(text: &str, doc: &Document, range: Range<usize>) -> Edited {
    let mut span = range.clone();
    // The formatting elements touching the span, growing it until none sticks out.
    let elements: Vec<Range<usize>> = doc
        .events
        .iter()
        .filter_map(|(e, r)| match e {
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Subscript | Tag::Superscript)
            | Event::Code(_)
            | Event::InlineHtml(_) => Some(r.clone()),
            _ => None,
        })
        .collect();
    loop {
        let before = span.clone();
        for r in &elements {
            if r.start < span.end && r.end > span.start {
                span.start = span.start.min(r.start);
                span.end = span.end.max(r.end);
            }
        }
        if span == before {
            break;
        }
    }
    let mut out = String::new();
    let mut pos = span.start;
    let mut i = 0;
    let events = &doc.events;
    while i < events.len() {
        let (event, r) = &events[i];
        if r.end <= span.start || r.start >= span.end || r.start < pos {
            i += 1;
            continue;
        }
        match event {
            Event::Text(t) => {
                // An escaping backslash the event leaves out is kept.
                if r.start > pos && text[..r.start].ends_with('\\') {
                    out.push('\\');
                }
                // Text with escapes is copied as written.
                if t.len() == r.len() {
                    out.push_str(&remove_highlight_marks(t));
                } else {
                    out.push_str(&text[r.clone()]);
                }
                pos = r.end;
            }
            Event::Code(t) => {
                out.push_str(t);
                pos = r.end;
            }
            Event::InlineHtml(_) => pos = r.end,
            Event::SoftBreak | Event::HardBreak | Event::InlineMath(_) | Event::FootnoteReference(_) => {
                out.push_str(&text[r.clone()]);
                pos = r.end;
            }
            Event::Start(Tag::Image { .. }) => {
                out.push_str(&text[r.clone()]);
                pos = r.end;
                i = end_of(events, i);
            }
            Event::Start(Tag::Link { .. }) => {
                // Keep the link: its text cleared, its destination as written.
                let end = end_of(events, i);
                let inner: Vec<_> = events[i + 1..end].iter().map(|(_, r)| r.clone()).collect();
                let text_end = inner.iter().map(|r| r.end).max().unwrap_or(r.start + 1);
                let inner_range = r.start + 1..text_end;
                let sub = Document::parse(&text[inner_range.clone()]);
                let cleared = clear_formatting(&text[inner_range.clone()], &sub, 0..inner_range.len()).text;
                let tail = &text[text_end..r.end];
                out.push('[');
                out.push_str(&cleared);
                out.push_str(tail.strip_prefix(']').map_or(tail, |_| tail));
                if !tail.starts_with(']') {
                    out.push(']');
                }
                pos = r.end;
                i = end;
            }
            _ => {}
        }
        i += 1;
    }
    let t = splice(text, span.clone(), &out);
    Edited { text: t, sel: Sel { anchor: span.start, head: span.start + out.len() }, pending: None }
}

fn end_of(events: &[(Event<'static>, Range<usize>)], start: usize) -> usize {
    let mut depth = 0;
    for (i, (e, _)) in events.iter().enumerate().skip(start) {
        match e {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    events.len() - 1
}

fn remove_highlight_marks(t: &str) -> String {
    if t.matches("==").count() >= 2 { t.replace("==", "") } else { t.to_string() }
}

/// The opening characters auto-pairing closes, with their closing character.
const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('"', '"'),
    ('\'', '\''),
    ('`', '`'),
    ('（', '）'),
    ('「', '」'),
    ('『', '』'),
    ('【', '】'),
];
/// The ones paired in code.
const CODE_PAIRS: &[char] = &['(', '[', '{', '"', '\''];
/// Characters that only enclose a selection.
const ENCLOSERS: &[char] = &['*', '_', '~', '$'];

fn closing(c: char) -> Option<char> {
    PAIRS.iter().find(|(o, _)| *o == c).map(|(_, cl)| *cl)
}

/// Typing `c` with auto-pairing: encloses a selection, closes an opening
/// character, or steps over a closing one. None types `c` as it is.
pub fn auto_pair(text: &str, sel: Sel, c: char, in_code: bool) -> Option<Edited> {
    let r = sel.range();
    let pairs_here = |c: char| !in_code || CODE_PAIRS.contains(&c);
    if !r.is_empty() {
        let close =
            closing(c).filter(|_| pairs_here(c)).or_else(|| (ENCLOSERS.contains(&c) && !in_code).then_some(c))?;
        let inner = &text[r.clone()];
        let t = splice(text, r.clone(), &format!("{c}{inner}{close}"));
        let s = r.start + c.len_utf8();
        return Some(Edited { text: t, sel: Sel { anchor: s, head: s + inner.len() }, pending: None });
    }
    let at = r.start;
    let next = text[at..].chars().next();
    let prev = text[..at].chars().next_back();
    // A closing character typed before the same one steps over it.
    // (In code, the closing character of a pair code pairs.)
    let opener = PAIRS.iter().find(|(_, cl)| *cl == c).map(|(o, _)| *o);
    if opener.is_some_and(pairs_here) && next == Some(c) {
        return Some(Edited { text: text.to_string(), sel: Sel::caret(at + c.len_utf8()), pending: None });
    }
    let close = closing(c).filter(|_| pairs_here(c))?;
    let symmetric = close == c;
    // Not inside a word (`don't`), nor before one.
    if next.is_some_and(char::is_alphanumeric) || (symmetric && prev.is_some_and(char::is_alphanumeric)) {
        return None;
    }
    let t = splice(text, at..at, &format!("{c}{close}"));
    Some(Edited { text: t, sel: Sel::caret(at + c.len_utf8()), pending: None })
}

/// Backspace between an empty pair deletes both characters.
pub fn pair_backspace(text: &str, at: usize) -> Option<Edited> {
    let prev = text[..at].chars().next_back()?;
    let next = text[at..].chars().next()?;
    (closing(prev) == Some(next)).then(|| {
        let t = splice(text, at - prev.len_utf8()..at + next.len_utf8(), "");
        Edited { text: t, sel: Sel::caret(at - prev.len_utf8()), pending: None }
    })
}

/// Checks or unchecks the task box of the list item whose first line is at `line_start`.
pub fn toggle_task(text: &str, line_start: usize) -> Option<Edited> {
    let line = line_range(text, line_start);
    let m = line_marker(&text[line.clone()])?;
    let task = m.task?;
    let mark = line.start + task.start + 1;
    let new = if text[mark..mark + 1].trim().is_empty() { "x" } else { " " };
    Some(Edited { text: splice(text, mark..mark + 1, new), sel: Sel::caret(line.start), pending: None })
}

/// The text inside the innermost formatted span (emphasis, strikethrough,
/// code, link, sub/superscript, highlight) holding `at`.
pub fn styled_span_at(text: &str, doc: &Document, at: usize) -> Option<Range<usize>> {
    let mut best: Option<Range<usize>> = None;
    let events = &doc.events;
    for (i, (event, r)) in events.iter().enumerate() {
        if !(r.start <= at && at <= r.end) {
            continue;
        }
        let inner = match event {
            Event::Start(
                Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Subscript | Tag::Superscript | Tag::Link { .. },
            ) => {
                let end = end_of(events, i);
                let rs: Vec<&Range<usize>> = events[i + 1..end].iter().map(|(_, r)| r).collect();
                match (rs.first(), rs.iter().map(|r| r.end).max()) {
                    (Some(f), Some(e)) => f.start..e,
                    _ => continue,
                }
            }
            Event::Code(_) => {
                let s = &text[r.clone()];
                let ticks = s.len() - s.trim_start_matches('`').len();
                r.start + ticks..r.end - ticks
            }
            _ => continue,
        };
        if best.as_ref().is_none_or(|b| inner.len() < b.len()) {
            best = Some(inner);
        }
    }
    // `==highlight==` within a text.
    let line = line_range(text, at);
    let l = &text[line.clone()];
    let marks: Vec<usize> = l.match_indices("==").map(|(i, _)| line.start + i).collect();
    for pair in marks.chunks(2) {
        if let [a, b] = pair
            && *a + 2 <= at
            && at <= *b
        {
            let inner = *a + 2..*b;
            if best.as_ref().is_none_or(|x| inner.len() < x.len()) {
                best = Some(inner);
            }
        }
    }
    let _ = TagEnd::Paragraph;
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `‸` marks the cursor; `«…»` a selection.
    fn parse(marked: &str) -> (String, Sel) {
        if let Some(a) = marked.find('«') {
            let b = marked.find('»').unwrap();
            let text = marked.replace(['«', '»'], "");
            let head = b - '«'.len_utf8();
            return (text, Sel { anchor: a, head });
        }
        let c = marked.find('‸').expect("cursor");
        (marked.replace('‸', ""), Sel::caret(c))
    }

    fn show(e: &Edited) -> String {
        let mut t = e.text.clone();
        if e.sel.is_empty() {
            t.insert(e.sel.head, '‸');
        } else {
            let r = e.sel.range();
            t.insert(r.end, '»');
            t.insert(r.start, '«');
        }
        t
    }

    fn toggle(marked: &str, style: InlineStyle) -> String {
        let (text, sel) = parse(marked);
        show(&toggle_inline(&text, sel, style))
    }

    #[test]
    fn bold_and_back() {
        assert_eq!(toggle("a «word» b", InlineStyle::Bold), "a **«word»** b");
        assert_eq!(toggle("a **«word»** b", InlineStyle::Bold), "a «word» b");
        assert_eq!(toggle("a «**word**» b", InlineStyle::Bold), "a «word» b");
        // The word at the cursor.
        assert_eq!(toggle("a wo‸rd b", InlineStyle::Bold), "a **«word»** b");
        assert_eq!(toggle("a ‸ b", InlineStyle::Bold), "a **‸** b");
    }

    #[test]
    fn italic_is_not_bold() {
        assert_eq!(toggle("**«w»**", InlineStyle::Italic), "***«w»***");
        assert_eq!(toggle("***«w»***", InlineStyle::Italic), "**«w»**");
        assert_eq!(toggle("*«w»*", InlineStyle::Bold), "***«w»***");
    }

    #[test]
    fn other_styles() {
        assert_eq!(toggle("«x»", InlineStyle::Underline), "<u>«x»</u>");
        assert_eq!(toggle("<u>«x»</u>", InlineStyle::Underline), "«x»");
        assert_eq!(toggle("«x»", InlineStyle::Code), "`«x»`");
        assert_eq!(toggle("«x»", InlineStyle::Strike), "~~«x»~~");
        assert_eq!(toggle("~~«x»~~", InlineStyle::Subscript), "~~~«x»~~~");
        assert_eq!(toggle("«x»", InlineStyle::Highlight), "==«x»==");
        assert_eq!(toggle("日本‸語のテキスト", InlineStyle::Bold).replace(['«', '»'], ""), "**日本語**のテキスト");
    }

    #[test]
    fn links() {
        let (t, s) = parse("see «docs» now");
        assert_eq!(show(&insert_link(&t, s)), "see [docs](‸) now");
        let (t, s) = parse("x‸");
        assert_eq!(show(&insert_link(&t, s)), "x[‸]()");
    }

    fn heading(marked: &str, level: u8) -> String {
        let (text, sel) = parse(marked);
        let doc = Document::parse(&text);
        let region = doc.blocks[doc.block_at(sel.head).unwrap()].range.clone();
        show(&set_heading(&text, &doc, region, sel, level).unwrap())
    }

    fn step(marked: &str, raise: bool) -> Option<String> {
        let (text, sel) = parse(marked);
        let doc = Document::parse(&text);
        let region = doc.blocks[doc.block_at(sel.head).unwrap()].range.clone();
        step_heading(&text, &doc, region, sel, raise).map(|e| show(&e))
    }

    #[test]
    fn headings() {
        assert_eq!(heading("Pl‸an", 2), "## Pl‸an");
        assert_eq!(heading("## Pl‸an", 0), "Pl‸an");
        assert_eq!(heading("# Pl‸an #", 3), "### Pl‸an");
        assert_eq!(heading("Ti‸tle\n===", 2), "## Ti‸tle");
        assert_eq!(heading("one\ntw‸o", 1), "# one tw‸o");
        assert_eq!(step("Pl‸an", true).as_deref(), Some("###### Pl‸an"));
        assert_eq!(step("### Pl‸an", true).as_deref(), Some("## Pl‸an"));
        assert_eq!(step("# Pl‸an", true).as_deref(), Some("# Pl‸an"));
        assert_eq!(step("###### Pl‸an", false).as_deref(), Some("Pl‸an"));
        assert_eq!(step("Pl‸an", false), None);
        // Not on other blocks.
        let (text, sel) = parse("- it‸em");
        let doc = Document::parse(&text);
        assert_eq!(set_heading(&text, &doc, doc.blocks[0].range.clone(), sel, 1), None);
    }

    #[test]
    fn quotes() {
        let (t, s) = parse("a‸\n\nb");
        let r = 0..t.len();
        assert_eq!(toggle_quote(&t, r.clone(), s).text, "> a\n>\n> b");
        let q = toggle_quote(&t, r, s).text;
        assert_eq!(toggle_quote(&q, 0..q.len(), s).text, "a\n\nb");
    }

    #[test]
    fn lists() {
        let text = "one\n\ntwo\nmore";
        let doc = Document::parse(text);
        let e = toggle_list(text, &doc, 0..text.len(), false);
        assert_eq!(e.text, "- one\n- two\n  more");
        let doc = Document::parse(&e.text);
        let e2 = toggle_list(&e.text, &doc, 0..e.text.len(), true);
        assert_eq!(e2.text, "1. one\n2. two\n   more");
        let doc = Document::parse(&e2.text);
        let e3 = toggle_list(&e2.text, &doc, 0..e2.text.len(), true);
        assert_eq!(e3.text, "one\n\ntwo\nmore");
    }

    #[test]
    fn clearing_formatting() {
        let clear = |text: &str| {
            let doc = Document::parse(text);
            clear_formatting(text, &doc, 0..text.len()).text
        };
        assert_eq!(clear("a **b** _c_ <u>d</u>"), "a b c d");
        assert_eq!(clear("x ~~s~~ `c` ==h== ^p^ ~q~"), "x s c h p q");
        assert_eq!(clear("keep [**the** link](http://x.y \"t\") here"), "keep [the link](http://x.y \"t\") here");
        assert_eq!(clear("escaped \\*star\\* stays"), "escaped \\*star\\* stays");
        // Only what the range touches.
        let text = "**a** b **c**";
        let doc = Document::parse(text);
        assert_eq!(clear_formatting(text, &doc, 2..3).text, "a b **c**");
    }

    fn pair(marked: &str, c: char, code: bool) -> Option<String> {
        let (text, sel) = parse(marked);
        auto_pair(&text, sel, c, code).map(|e| show(&e))
    }

    #[test]
    fn auto_pairing() {
        assert_eq!(pair("a ‸", '(', false).as_deref(), Some("a (‸)"));
        assert_eq!(pair("(x‸)", ')', false).as_deref(), Some("(x)‸"));
        assert_eq!(pair("‸", '「', false).as_deref(), Some("「‸」"));
        assert_eq!(pair("«word»", '*', false).as_deref(), Some("*«word»*"));
        assert_eq!(pair("«word»", '(', false).as_deref(), Some("(«word»)"));
        assert_eq!(pair("‸", '*', false), None);
        // Not inside words.
        assert_eq!(pair("don‸", '\'', false), None);
        assert_eq!(pair("‸word", '(', false), None);
        // In code only some.
        assert_eq!(pair("‸", '`', true), None);
        assert_eq!(pair("‸", '[', true).as_deref(), Some("[‸]"));
        assert_eq!(pair("f(x‸)", ')', true).as_deref(), Some("f(x)‸"));
        let (t, s) = parse("a(‸)b");
        assert_eq!(show(&pair_backspace(&t, s.head).unwrap()), "a‸b");
        assert!(pair_backspace("a(b", 2).is_none());
    }

    #[test]
    fn tasks() {
        assert_eq!(toggle_task("- [ ] call", 0).unwrap().text, "- [x] call");
        assert_eq!(toggle_task("a\n  1. [x] b", 2).unwrap().text, "a\n  1. [ ] b");
        assert!(toggle_task("- call", 0).is_none());
    }

    #[test]
    fn selection_helpers() {
        let text = "some **bold text** and `code` ==hi== [link](u)";
        let doc = Document::parse(text);
        let at = |s: &str| text.find(s).unwrap() + 1;
        assert_eq!(&text[styled_span_at(text, &doc, at("old")).unwrap()], "bold text");
        assert_eq!(&text[styled_span_at(text, &doc, at("ode")).unwrap()], "code");
        assert_eq!(&text[styled_span_at(text, &doc, at("hi")).unwrap()], "hi");
        assert_eq!(&text[styled_span_at(text, &doc, at("ink")).unwrap()], "link");
        assert_eq!(styled_span_at(text, &doc, 1), None);
        assert_eq!(&text[word_at(text, 2).unwrap()], "some");
        assert_eq!(&text[word_at(text, 4).unwrap()], "some");
        let jp = "日本語のテキスト";
        assert_eq!(&jp[word_at(jp, jp.find("ス").unwrap()).unwrap()], "テキスト");
        assert_eq!(word_at("a  b", 2), None);
    }
}
