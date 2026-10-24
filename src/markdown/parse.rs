//! A page parsed into blocks: the unit the editor shows rendered, or as
//! source while the cursor is in it.
//!
//! The whole text is parsed at once (pulldown-cmark, with byte offsets), so
//! that reference links and footnotes resolve across blocks. Top-level
//! elements become blocks; lists are flattened so that every list item is a
//! block of its own (without its nested lists). Text that no element covers
//! (link reference definitions) becomes an `Other` block, so that every
//! non-blank byte of the page belongs to exactly one block.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::ops::Range;

/// The Markdown dialect of the editor: CommonMark with GitHub's tables,
/// footnotes, strikethrough and task lists, front matter, math and
/// Typora's sub/superscript.
pub fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_SUBSCRIPT
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// From the start of its first line (indentation included) to the end of
    /// its last non-blank character.
    pub range: Range<usize>,
    pub kind: BlockKind,
    /// Its events in `Document::events`, without the enclosing start and end
    /// of a top-level element (the inline content of a paragraph, the
    /// contents of a quote, the rows of a table...). Never contains a nested
    /// list of a list item.
    pub events: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    Paragraph,
    /// A paragraph that is only `$$ ... $$`.
    Math,
    /// A paragraph that is only `[toc]`.
    Toc,
    Heading(u8),
    ThematicBreak,
    Code {
        /// The info string's first word of a fenced block.
        lang: Option<String>,
        fenced: bool,
    },
    Table,
    Quote,
    ListItem(ListItem),
    Html,
    FrontMatter,
    Footnote(String),
    /// Text no element covers, such as link reference definitions.
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// 0 for an item of a top-level list.
    pub depth: usize,
    /// The number shown for an item of an ordered list.
    pub number: Option<u64>,
    /// Whether it is checked, for a task item.
    pub task: Option<bool>,
    /// Whether the list separates its items with blank lines.
    pub loose: bool,
    /// The item's content after one of its nested lists, not the item itself.
    pub continuation: bool,
    /// The end of the item with all its nested lists.
    pub subtree_end: usize,
}

impl BlockKind {
    /// Whether its source is inline text a user types into, which can be
    /// joined with a neighbor.
    pub fn is_text(&self) -> bool {
        matches!(
            self,
            BlockKind::Paragraph | BlockKind::Heading(_) | BlockKind::ListItem(_) | BlockKind::Other | BlockKind::Toc
        )
    }

    /// Code-like blocks: their text is taken as it is, no inline syntax.
    pub fn is_code(&self) -> bool {
        matches!(self, BlockKind::Code { .. } | BlockKind::Math | BlockKind::FrontMatter | BlockKind::Html)
    }

    pub fn is_mermaid(&self) -> bool {
        matches!(self, BlockKind::Code { lang: Some(l), .. } if l.eq_ignore_ascii_case("mermaid"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadingInfo {
    pub block: usize,
    pub level: u8,
    pub text: String,
    /// The anchor `[t](#anchor)` links to it with.
    pub anchor: String,
}

/// A parsed page.
#[derive(Debug, Clone)]
pub struct Document {
    pub text: String,
    /// The events with their byte ranges in `text`.
    pub events: Vec<(Event<'static>, Range<usize>)>,
    pub blocks: Vec<Block>,
    /// Footnote labels by the number they are shown with (order of first reference).
    pub footnotes: HashMap<String, usize>,
    pub headings: Vec<HeadingInfo>,
}

impl Document {
    pub fn parse(text: &str) -> Document {
        let prepared = lenient_links(text);
        let parser = Parser::new_ext(&prepared.text, options());
        let events: Vec<(Event<'static>, Range<usize>)> = parser
            .into_offset_iter()
            .map(|(e, r)| (e.into_static(), prepared.original(r.start)..prepared.original(r.end)))
            .collect();
        let mut doc = Document {
            text: text.to_string(),
            events,
            blocks: Vec::new(),
            footnotes: HashMap::new(),
            headings: Vec::new(),
        };
        doc.blocks = segment(&doc.text, &doc.events);
        doc.footnotes = number_footnotes(&doc.events);
        doc.headings = headings(&doc);
        doc
    }

    /// The index of the block containing byte `offset` (its end included).
    pub fn block_at(&self, offset: usize) -> Option<usize> {
        self.blocks.iter().position(|b| b.range.start <= offset && offset <= b.range.end)
    }

    /// The source of a block.
    pub fn source(&self, block: usize) -> &str {
        &self.text[self.blocks[block].range.clone()]
    }

    pub fn block_events(&self, block: usize) -> &[(Event<'static>, Range<usize>)] {
        &self.events[self.blocks[block].events.clone()]
    }

    /// The heading an anchor (`#anchor`, already without `#`) refers to.
    pub fn heading_for_anchor(&self, anchor: &str) -> Option<&HeadingInfo> {
        let decoded = percent_encoding::percent_decode_str(anchor).decode_utf8_lossy();
        let wanted = decoded.to_lowercase();
        self.headings
            .iter()
            .find(|h| h.anchor == wanted)
            .or_else(|| self.headings.iter().find(|h| h.text.to_lowercase() == wanted))
    }
}

/// The text a parser sees, with the bytes inserted to make it parse leniently.
struct Prepared {
    text: String,
    /// Positions in `text` of the inserted bytes, ascending.
    inserted: Vec<usize>,
}

impl Prepared {
    /// The offset in the original text of an offset in the prepared one.
    fn original(&self, offset: usize) -> usize {
        offset - self.inserted.partition_point(|&p| p < offset)
    }
}

/// Wraps link destinations that contain spaces (`[a](My Page.assets)`, as
/// the page template writes them) in angle brackets, which CommonMark
/// requires there. Code is left alone.
fn lenient_links(text: &str) -> Prepared {
    let wraps = spaced_destinations(text);
    if wraps.is_empty() {
        return Prepared { text: text.to_string(), inserted: Vec::new() };
    }
    let mut out = String::with_capacity(text.len() + wraps.len() * 2);
    let mut inserted = Vec::new();
    let mut pos = 0;
    for r in wraps {
        out.push_str(&text[pos..r.start]);
        inserted.push(out.len());
        out.push('<');
        out.push_str(&text[r.clone()]);
        inserted.push(out.len());
        out.push('>');
        pos = r.end;
    }
    out.push_str(&text[pos..]);
    Prepared { text: out, inserted }
}

/// The destinations with spaces of `[...](...)` that CommonMark doesn't take
/// as links, outside code.
fn spaced_destinations(text: &str) -> Vec<Range<usize>> {
    if !text.contains("](") {
        return Vec::new();
    }
    let code = code_ranges(text);
    let in_code = |i: usize| code.iter().any(|r| r.contains(&i));
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(i) = text[search..].find("](").map(|i| i + search) {
        search = i + 2;
        if in_code(i) || !has_opening_bracket(text, i) {
            continue;
        }
        let line_end = text[i..].find('\n').map_or(text.len(), |e| i + e);
        // The parenthesized part, parentheses balanced, on the same line.
        let start = i + 2;
        let mut depth = 0;
        let mut close = None;
        let mut j = start;
        while j < line_end {
            match bytes[j] {
                b'\\' => j += 1,
                b'(' => depth += 1,
                b')' if depth == 0 => {
                    close = Some(j);
                    break;
                }
                b')' => depth -= 1,
                _ => {}
            }
            j += 1;
        }
        let Some(close) = close else { continue };
        let inner = &text[start..close];
        let lead = inner.len() - inner.trim_start().len();
        let dest_start = start + lead;
        let mut dest_end = start + inner.trim_end().len();
        if dest_start >= dest_end || bytes[dest_start] == b'<' {
            continue;
        }
        // A title at the end: ` "..."`, ` '...'` or ` (...)`.
        let body = &text[dest_start..dest_end];
        let last = body.as_bytes()[body.len() - 1];
        let opener = match last {
            b'"' => Some('"'),
            b'\'' => Some('\''),
            b')' => Some('('),
            _ => None,
        };
        if let Some(opener) = opener
            && let Some(o) = body[..body.len() - 1].rfind(opener)
            && o > 0
            && body.as_bytes()[o - 1].is_ascii_whitespace()
        {
            dest_end = dest_start + body[..o].trim_end().len();
        }
        let dest = &text[dest_start..dest_end];
        if dest.contains(' ') && !dest.contains(['<', '>']) {
            found.push(dest_start..dest_end);
        }
        search = close;
    }
    found
}

/// Whether `]` at `close` has an unescaped `[` before it on its line.
fn has_opening_bracket(text: &str, close: usize) -> bool {
    let line_start = text[..close].rfind('\n').map_or(0, |i| i + 1);
    let mut depth = 0;
    for (i, c) in text[line_start..close].char_indices().rev() {
        let escaped = text[line_start..line_start + i].ends_with('\\');
        match c {
            ']' if !escaped => depth += 1,
            '[' if !escaped => {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    false
}

/// The byte ranges of code blocks and code spans.
fn code_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut code_block: Option<usize> = None;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = code_block.take() {
                    ranges.push(start..range.end);
                }
            }
            Event::Code(_) | Event::InlineMath(_) | Event::DisplayMath(_) | Event::Html(_) | Event::InlineHtml(_) => {
                ranges.push(range)
            }
            _ => {}
        }
    }
    ranges
}

/// The start of the line holding `offset` when only whitespace precedes it there.
fn line_start_if_indented(text: &str, offset: usize) -> usize {
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    if text[line_start..offset].chars().all(|c| c == ' ' || c == '\t') { line_start } else { offset }
}

/// `range` without trailing whitespace.
fn trim_end(text: &str, range: Range<usize>) -> Range<usize> {
    let end = range.start + text[range.clone()].trim_end().len();
    range.start..end
}

/// The index of the event ending the element started at `start`.
fn matching_end(events: &[(Event<'static>, Range<usize>)], start: usize) -> usize {
    let mut depth = 0;
    for (i, (event, _)) in events.iter().enumerate().skip(start) {
        match event {
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

fn segment(text: &str, events: &[(Event<'static>, Range<usize>)]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < events.len() {
        let (event, range) = &events[i];
        match event {
            Event::Start(tag) => {
                let end = matching_end(events, i);
                if let Tag::List(start) = tag {
                    flatten_list(text, events, i, end, *start, 0, &mut blocks);
                } else {
                    let kind = block_kind(tag, &events[i + 1..end], text, range);
                    let range = trim_end(text, line_start_if_indented(text, range.start)..range.end);
                    blocks.push(Block { range, kind, events: i + 1..end });
                }
                i = end + 1;
            }
            Event::Rule => {
                let range = trim_end(text, line_start_if_indented(text, range.start)..range.end);
                blocks.push(Block { range, kind: BlockKind::ThematicBreak, events: i..i });
                i += 1;
            }
            _ => {
                // Top-level HTML outside an HTML block does not occur; skip anything else.
                i += 1;
            }
        }
    }
    blocks.sort_by_key(|b| b.range.start);
    fill_gaps(text, blocks)
}

fn block_kind(tag: &Tag, inner: &[(Event<'static>, Range<usize>)], text: &str, range: &Range<usize>) -> BlockKind {
    match tag {
        Tag::Paragraph => {
            let content: Vec<&Event> =
                inner.iter().map(|(e, _)| e).filter(|e| !matches!(e, Event::SoftBreak | Event::HardBreak)).collect();
            let blank = |e: &Event| matches!(e, Event::Text(t) if t.trim().is_empty());
            let non_blank: Vec<&&Event> = content.iter().filter(|e| !blank(e)).collect();
            if non_blank.len() == 1 && matches!(non_blank[0], Event::DisplayMath(_)) {
                BlockKind::Math
            } else if text[range.clone()].trim().eq_ignore_ascii_case("[toc]") {
                BlockKind::Toc
            } else {
                BlockKind::Paragraph
            }
        }
        Tag::Heading { level, .. } => BlockKind::Heading(heading_level(*level)),
        Tag::BlockQuote(_) => BlockKind::Quote,
        Tag::CodeBlock(CodeBlockKind::Fenced(info)) => BlockKind::Code {
            lang: info.split_whitespace().next().map(str::to_string).filter(|l| !l.is_empty()),
            fenced: true,
        },
        Tag::CodeBlock(CodeBlockKind::Indented) => BlockKind::Code { lang: None, fenced: false },
        Tag::HtmlBlock => BlockKind::Html,
        Tag::Table(_) => BlockKind::Table,
        Tag::MetadataBlock(_) => BlockKind::FrontMatter,
        Tag::FootnoteDefinition(label) => BlockKind::Footnote(label.to_string()),
        _ => BlockKind::Other,
    }
}

pub fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Adds a block for every item of the list starting at event `start` (and
/// ending at `end`), and recursively for its nested lists.
fn flatten_list(
    text: &str,
    events: &[(Event<'static>, Range<usize>)],
    start: usize,
    end: usize,
    first_number: Option<u64>,
    depth: usize,
    blocks: &mut Vec<Block>,
) {
    // A loose list has paragraphs in its items.
    let loose = {
        let mut level = 0;
        let mut loose = false;
        for (event, _) in &events[start + 1..end] {
            match event {
                Event::Start(Tag::Item) => level += 1,
                Event::End(TagEnd::Item) => level -= 1,
                Event::Start(Tag::Paragraph) if level == 1 => loose = true,
                Event::Start(Tag::List(_)) => level += 10,
                Event::End(TagEnd::List(_)) => level -= 10,
                _ => {}
            }
        }
        loose
    };
    let mut index = 0;
    let mut i = start + 1;
    while i < end {
        if !matches!(events[i].0, Event::Start(Tag::Item)) {
            i += 1;
            continue;
        }
        let item_end = matching_end(events, i);
        let item_range = events[i].1.clone();
        let number = first_number.map(|n| n + index);
        index += 1;
        let task = events[i + 1..item_end].iter().take(3).find_map(|(e, _)| match e {
            Event::TaskListMarker(checked) => Some(*checked),
            _ => None,
        });
        // The item's own segments around its nested lists.
        let mut segment_start = line_start_if_indented(text, item_range.start);
        let mut segment_events = i + 1;
        let mut continuation = false;
        let mut j = i + 1;
        let push = |range: Range<usize>, evs: Range<usize>, continuation: bool, blocks: &mut Vec<Block>| {
            // A continuation starts at its first non-blank line.
            let start = if continuation {
                let skipped = text[range.clone()].len() - text[range.clone()].trim_start().len();
                line_start_if_indented(text, range.start + skipped)
            } else {
                range.start
            };
            let range = trim_end(text, start.min(range.end)..range.end);
            if !continuation || !text[range.clone()].trim().is_empty() {
                let item = ListItem { depth, number, task, loose, continuation, subtree_end: item_range.end };
                blocks.push(Block { range, kind: BlockKind::ListItem(item), events: evs });
            }
        };
        while j < item_end {
            if let Event::Start(Tag::List(n)) = &events[j].0 {
                let list_end = matching_end(events, j);
                let list_start = line_start_if_indented(text, events[j].1.start);
                push(segment_start..list_start, segment_events..j, continuation, blocks);
                flatten_list(text, events, j, list_end, *n, depth + 1, blocks);
                continuation = true;
                segment_start = events[j].1.end;
                segment_events = list_end + 1;
                j = list_end + 1;
            } else {
                j += 1;
            }
        }
        push(segment_start..item_range.end, segment_events..item_end, continuation, blocks);
        i = item_end + 1;
    }
}

/// Adds an `Other` block for each run of non-blank lines no block covers.
fn fill_gaps(text: &str, blocks: Vec<Block>) -> Vec<Block> {
    let mut out = Vec::with_capacity(blocks.len());
    let mut pos = 0;
    let gap_blocks = |from: usize, to: usize, out: &mut Vec<Block>| {
        // Non-blank lines, grouped into runs separated by blank lines.
        let mut run: Option<Range<usize>> = None;
        let mut line_start = from;
        for line in text[from..to].split_inclusive('\n') {
            let line_end = line_start + line.len();
            if line.trim().is_empty() {
                if let Some(r) = run.take() {
                    out.push(Block { range: trim_end(text, r), kind: BlockKind::Other, events: 0..0 });
                }
            } else {
                run = Some(run.map_or(line_start..line_end, |r| r.start..line_end));
            }
            line_start = line_end;
        }
        if let Some(r) = run {
            out.push(Block { range: trim_end(text, r), kind: BlockKind::Other, events: 0..0 });
        }
    };
    for block in blocks {
        if block.range.start > pos {
            gap_blocks(pos, block.range.start, &mut out);
        }
        pos = pos.max(block.range.end);
        out.push(block);
    }
    if pos < text.len() {
        gap_blocks(pos, text.len(), &mut out);
    }
    out
}

/// Footnote numbers in the order of their first reference.
fn number_footnotes(events: &[(Event<'static>, Range<usize>)]) -> HashMap<String, usize> {
    let mut numbers = HashMap::new();
    for (event, _) in events {
        if let Event::FootnoteReference(label) = event {
            let next = numbers.len() + 1;
            numbers.entry(label.to_string()).or_insert(next);
        }
    }
    numbers
}

fn headings(doc: &Document) -> Vec<HeadingInfo> {
    let mut out = Vec::new();
    let mut used: HashMap<String, usize> = HashMap::new();
    for (i, block) in doc.blocks.iter().enumerate() {
        let BlockKind::Heading(level) = block.kind else { continue };
        let text = plain_text(doc.block_events(i));
        let base = anchor(&text);
        // Repeated headings get `-1`, `-2`... as on GitHub.
        let n = used.entry(base.clone()).or_insert(0);
        let anchor = if *n == 0 { base.clone() } else { format!("{base}-{n}") };
        *n += 1;
        out.push(HeadingInfo { block: i, level, text, anchor });
    }
    out
}

/// The text of inline events, without markup.
pub fn plain_text(events: &[(Event<'static>, Range<usize>)]) -> String {
    let mut out = String::new();
    for (event, _) in events {
        match event {
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) => out.push_str(t),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out
}

/// The anchor of a heading: lower case, spaces as `-`, punctuation dropped (as GitHub does).
pub fn anchor(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            '-' | '_' => Some(c),
            c if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(String, BlockKind)> {
        let doc = Document::parse(text);
        doc.blocks.iter().map(|b| (text[b.range.clone()].to_string(), b.kind.clone())).collect()
    }

    fn sources(text: &str) -> Vec<String> {
        kinds(text).into_iter().map(|(s, _)| s).collect()
    }

    #[test]
    fn top_level_blocks() {
        let text = "---\ntitle: x\n---\n\n# Head\n\npara\nline2\n\n> q1\n> > q2\n\n| a | b |\n|---|:-:|\n| 1 | 2 |\n\n$$\nx^2\n$$\n\n[^1]: note\n\n[id]: http://x\n\n```rust\nfn main() {}\n```\n<div>\nhtml\n</div>\n\n[toc]\n\n---\n";
        let k = kinds(text);
        let expected: Vec<(&str, BlockKind)> = vec![
            ("---\ntitle: x\n---", BlockKind::FrontMatter),
            ("# Head", BlockKind::Heading(1)),
            ("para\nline2", BlockKind::Paragraph),
            ("> q1\n> > q2", BlockKind::Quote),
            ("| a | b |\n|---|:-:|\n| 1 | 2 |", BlockKind::Table),
            ("$$\nx^2\n$$", BlockKind::Math),
            ("[^1]: note", BlockKind::Footnote("1".into())),
            ("[id]: http://x", BlockKind::Other),
            ("```rust\nfn main() {}\n```", BlockKind::Code { lang: Some("rust".into()), fenced: true }),
            ("<div>\nhtml\n</div>", BlockKind::Html),
            ("[toc]", BlockKind::Toc),
            ("---", BlockKind::ThematicBreak),
        ];
        assert_eq!(k.len(), expected.len(), "{k:#?}");
        for ((source, kind), (es, ek)) in k.iter().zip(expected) {
            assert_eq!((source.as_str(), kind), (es, &ek));
        }
    }

    #[test]
    fn list_items_are_blocks_of_their_own() {
        let text = "- a\n  - b\n    - c\n- [ ] task\n\n3. one\n1. two\n";
        let doc = Document::parse(text);
        let items: Vec<(&str, usize, Option<u64>, Option<bool>)> = doc
            .blocks
            .iter()
            .map(|b| match &b.kind {
                BlockKind::ListItem(i) => (&text[b.range.clone()], i.depth, i.number, i.task),
                k => panic!("{k:?}"),
            })
            .collect();
        assert_eq!(
            items,
            vec![
                ("- a", 0, None, None),
                ("  - b", 1, None, None),
                ("    - c", 2, None, None),
                ("- [ ] task", 0, None, Some(false)),
                ("3. one", 0, Some(3), None),
                ("1. two", 0, Some(4), None),
            ]
        );
        // The item's events leave its nested list out.
        let a = doc.block_events(0);
        assert!(a.iter().all(|(e, _)| !matches!(e, Event::Start(Tag::List(_)))));
        assert_eq!(plain_text(a), "a");
    }

    #[test]
    fn content_after_a_nested_list_is_a_continuation() {
        let text = "- a\n\n  - b\n\n  more\n- c\n";
        let doc = Document::parse(text);
        let s: Vec<(&str, bool)> = doc
            .blocks
            .iter()
            .map(|b| match &b.kind {
                BlockKind::ListItem(i) => (&text[b.range.clone()], i.continuation),
                k => panic!("{k:?}"),
            })
            .collect();
        assert_eq!(s, vec![("- a", false), ("  - b", false), ("  more", true), ("- c", false)]);
    }

    #[test]
    fn loose_lists() {
        let doc = Document::parse("- a\n\n- b\n");
        assert!(matches!(&doc.blocks[0].kind, BlockKind::ListItem(i) if i.loose));
        let doc = Document::parse("- a\n- b\n");
        assert!(matches!(&doc.blocks[0].kind, BlockKind::ListItem(i) if !i.loose));
    }

    #[test]
    fn destinations_with_spaces_are_links() {
        let text = "[assets](My Page.assets)\n\n![x](My Page.assets/a b.png \"t\") `[c](d e)`\n";
        let doc = Document::parse(text);
        let links: Vec<String> = doc
            .events
            .iter()
            .filter_map(|(e, _)| match e {
                Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => Some(dest_url.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(links, vec!["My Page.assets", "My Page.assets/a b.png"]);
        // Offsets are those of the original text.
        assert_eq!(sources(text), vec!["[assets](My Page.assets)", "![x](My Page.assets/a b.png \"t\") `[c](d e)`"]);
        let code = doc.events.iter().find_map(|(e, r)| matches!(e, Event::Code(_)).then(|| r.clone())).unwrap();
        assert_eq!(&text[code], "`[c](d e)`");
    }

    #[test]
    fn brackets_without_a_link_stay_text() {
        let doc = Document::parse("see ](a b) here\n");
        assert_eq!(plain_text(doc.block_events(0)), "see ](a b) here");
    }

    #[test]
    fn headings_and_anchors() {
        let doc = Document::parse("# Hello World!\n\n## 日本語 見出し\n\n# Hello World!\n");
        let h: Vec<(u8, &str, &str)> =
            doc.headings.iter().map(|h| (h.level, h.text.as_str(), h.anchor.as_str())).collect();
        assert_eq!(
            h,
            vec![
                (1, "Hello World!", "hello-world"),
                (2, "日本語 見出し", "日本語-見出し"),
                (1, "Hello World!", "hello-world-1")
            ]
        );
        assert_eq!(doc.heading_for_anchor("%E6%97%A5%E6%9C%AC%E8%AA%9E-%E8%A6%8B%E5%87%BA%E3%81%97").unwrap().level, 2);
    }

    #[test]
    fn setext_heading_and_indented_code() {
        let k = kinds("Title\n=====\n\n    code\n");
        assert_eq!(k[0], ("Title\n=====".to_string(), BlockKind::Heading(1)));
        assert_eq!(k[1], ("    code".to_string(), BlockKind::Code { lang: None, fenced: false }));
    }

    #[test]
    fn footnotes_are_numbered_by_first_reference() {
        let doc = Document::parse("b[^b] a[^a] b[^b]\n\n[^a]: A\n[^b]: B\n");
        assert_eq!(doc.footnotes["b"], 1);
        assert_eq!(doc.footnotes["a"], 2);
    }

    #[test]
    fn block_at_includes_the_end() {
        let doc = Document::parse("one\n\ntwo\n");
        assert_eq!(doc.block_at(0), Some(0));
        assert_eq!(doc.block_at(3), Some(0));
        assert_eq!(doc.block_at(4), None);
        assert_eq!(doc.block_at(5), Some(1));
    }
}
