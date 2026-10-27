//! A page parsed into blocks: the unit the editor shows rendered, or as
//! source while the cursor is in it.
//!
//! The whole text is parsed at once (pulldown-cmark, with byte offsets), so
//! that reference links and footnotes resolve across blocks. Top-level
//! elements become blocks; lists are flattened so that every list item is a
//! block of its own (without its nested lists). Quotes are containers: each
//! block in a quote is a block of its own, and so is each block of a list
//! item after its first one; the containers around a block are its `outer`.
//! Text that no element covers (link reference definitions) becomes an
//! `Other` block, so that every non-blank byte of the page outside container
//! syntax belongs to exactly one block.

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
    /// The containers around it (indices in `Document::containers`), outermost first.
    pub outer: Vec<usize>,
}

/// A quote, or a list item holding blocks after its first one: its blocks
/// are blocks of their own, with their lines starting with its prefix.
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub kind: ContainerKind,
    /// From the start of its first line to its last non-blank character.
    pub range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    /// Its lines start with `>` (and a space).
    Quote,
    /// Its lines after the first are indented by `indent` spaces (its
    /// content column, in its lines without the prefixes of the containers
    /// around it); `depth` is that of its block.
    Item { depth: usize, indent: usize },
}

impl ContainerKind {
    /// What it puts at the start of a line of its content, and of a blank one.
    pub fn line_prefix(self, blank: bool) -> String {
        match (self, blank) {
            (ContainerKind::Quote, false) => "> ".into(),
            (ContainerKind::Quote, true) => ">".into(),
            (ContainerKind::Item { indent, .. }, false) => " ".repeat(indent),
            (ContainerKind::Item { .. }, true) => String::new(),
        }
    }
}

/// The length of the prefixes the containers `kinds` (outermost first) put
/// at the start of `line`: a quote's `>` (after up to three spaces) with the
/// space after it, a list item's indentation up to its content column. A
/// line missing a quote's `>` (a lazy continuation line) ends there.
pub fn prefix_len(line: &str, kinds: impl IntoIterator<Item = ContainerKind>) -> usize {
    let b = line.as_bytes();
    let mut pos = 0;
    for kind in kinds {
        match kind {
            ContainerKind::Quote => {
                let spaces = b[pos..].iter().take(3).take_while(|&&c| c == b' ').count();
                if b.get(pos + spaces) != Some(&b'>') {
                    return pos;
                }
                pos += spaces + 1;
                if matches!(b.get(pos), Some(b' ' | b'\t')) {
                    pos += 1;
                }
            }
            ContainerKind::Item { indent, .. } => {
                pos += b[pos..].iter().take(indent).take_while(|&&c| c == b' ').count();
            }
        }
    }
    pos
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
    /// The end of the item with all its nested lists.
    pub subtree_end: usize,
    /// Where its top-level list starts: items of one list share it.
    pub list: usize,
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
    pub containers: Vec<Container>,
    /// Footnote labels by the number they are shown with (order of first reference).
    pub footnotes: HashMap<String, usize>,
    pub headings: Vec<HeadingInfo>,
}

impl Document {
    pub fn parse(text: &str) -> Document {
        Document::parse_masked(text, None)
    }

    /// Parses `text` as if the byte at `mask` (an ASCII character other than
    /// a line break) were a letter, so that a marker being typed there is
    /// not taken as syntax; the text the parse shows there is the real one.
    pub fn parse_masked(text: &str, mask: Option<usize>) -> Document {
        let mask = mask.filter(|&m| text.as_bytes().get(m).is_some_and(|b| b.is_ascii() && *b != b'\n'));
        let doc = Document::build(text, mask, &[]);
        // An empty item starting a nested list (just indented with Tab) is
        // not an item to CommonMark: it cannot interrupt the paragraph above.
        let empty = empty_nested_items(&doc);
        if empty.is_empty() {
            return doc;
        }
        let items = Document::build(text, mask, &empty);
        let all_items = empty.iter().all(|&at| {
            let line = line_start(text, at);
            items
                .blocks
                .iter()
                .any(|b| matches!(b.kind, BlockKind::ListItem(_)) && line_start(text, b.range.start) == line)
        });
        if all_items { items } else { doc }
    }

    /// Parses `text` with a letter after each of the `fills` (the ends of
    /// lines), which the events' ranges do not show.
    fn build(text: &str, mask: Option<usize>, fills: &[usize]) -> Document {
        let mut base = text.to_string();
        if let Some(m) = mask {
            base.replace_range(m..=m, "x");
        }
        for &at in fills.iter().rev() {
            base.insert(at, 'x');
        }
        let prepared = lenient_links(&base);
        // Fill `i` is the byte at `fills[i] + i` of the text the parser got.
        let fill_at = |i: usize| fills[i] + i;
        let in_text = |offset: usize| {
            let offset = prepared.original(offset);
            offset - (0..fills.len()).filter(|&i| fill_at(i) < offset).count()
        };
        let is_fill = |offset: usize, end: usize| {
            let (start, end) = (prepared.original(offset), prepared.original(end));
            (0..fills.len()).any(|i| start == fill_at(i) && end == start + 1)
        };
        let mut events: Vec<(Event<'static>, Range<usize>)> = Vec::new();
        for (event, range) in Parser::new_ext(&prepared.text, options()).into_offset_iter() {
            let letter = matches!(event, Event::Text(_) | Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph));
            if !(letter && is_fill(range.start, range.end)) {
                events.push((event.into_static(), in_text(range.start)..in_text(range.end)));
            }
        }
        if let Some(m) = mask {
            for (event, range) in &mut events {
                if let Event::Text(t) = event
                    && range.contains(&m)
                    && t.len() == range.len()
                {
                    *event = Event::Text(text[range.clone()].to_string().into());
                }
            }
        }
        let mut doc = Document {
            text: text.to_string(),
            events,
            blocks: Vec::new(),
            containers: Vec::new(),
            footnotes: HashMap::new(),
            headings: Vec::new(),
        };
        (doc.blocks, doc.containers) = segment(&doc.text, &doc.events);
        doc.footnotes = number_footnotes(&doc.events);
        doc.headings = headings(&doc);
        doc
    }

    /// The kinds of the containers around block `block`, outermost first.
    pub fn outer_kinds(&self, block: usize) -> Vec<ContainerKind> {
        self.blocks.get(block).map_or(Vec::new(), |b| b.outer.iter().map(|&c| self.containers[c].kind).collect())
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

/// The start of the line of `text` holding byte `at`.
fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// The ends of the lines that are nothing but the marker of a list item
/// (`*`, `+`, `-`, `1.`, `1)`), indented to nest under the item above them,
/// when `doc` has them as lines of that item's paragraph.
fn empty_nested_items(doc: &Document) -> Vec<usize> {
    let text = &doc.text;
    let mut found = Vec::new();
    let mut start = 0;
    for line in text.split('\n') {
        let at = start;
        start += line.len() + 1;
        let indent = line.len() - line.trim_start_matches(' ').len();
        let rest = line[indent..].trim_end_matches([' ', '\t']);
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let marker =
            matches!(rest, "*" | "+" | "-") || (digits > 0 && digits <= 9 && matches!(&rest[digits..], "." | ")"));
        if !marker || at < 2 || text[..at - 1].ends_with('\n') || line.ends_with('\r') {
            continue;
        }
        // In the paragraph of an item whose text starts to the left of it.
        let Some(item) = doc
            .blocks
            .iter()
            .find(|b| matches!(b.kind, BlockKind::ListItem(_)) && b.range.start < at && at < b.range.end)
        else {
            continue;
        };
        let first = &text[line_start(text, item.range.start)..];
        let first = first.split('\n').next().unwrap_or_default();
        if crate::markdown::edit::line_marker(first).is_some_and(|m| indent >= m.end) {
            found.push(at + line.len());
        }
    }
    found
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

/// The start of the line holding `offset` when only whitespace (and, inside
/// a container, the `>` of quotes) precedes it there.
fn line_start_if_indented(text: &str, offset: usize, inside: bool) -> usize {
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let prefix = |c: char| c == ' ' || c == '\t' || (inside && c == '>');
    if text[line_start..offset].chars().all(prefix) { line_start } else { offset }
}

/// Whether an event starts a block (or is a thematic break).
fn is_block(e: &Event) -> bool {
    matches!(
        e,
        Event::Rule
            | Event::Start(
                Tag::Paragraph
                    | Tag::Heading { .. }
                    | Tag::BlockQuote(_)
                    | Tag::CodeBlock(_)
                    | Tag::HtmlBlock
                    | Tag::Table(_)
                    | Tag::MetadataBlock(_)
                    | Tag::FootnoteDefinition(_)
                    | Tag::List(_)
            )
    )
}

/// Whether `s` holds nothing but whitespace (and, inside a container, `>`).
fn is_blank(s: &str, inside: bool) -> bool {
    s.chars().all(|c| c.is_whitespace() || (inside && c == '>'))
}

/// `range` without trailing whitespace, nor trailing lines holding nothing
/// but the prefixes of the containers `kinds` around it.
fn trim_end(text: &str, range: Range<usize>, kinds: &[ContainerKind]) -> Range<usize> {
    let mut end = range.start + text[range.clone()].trim_end().len();
    while !kinds.is_empty() {
        let Some(nl) = text[range.start..end].rfind('\n') else { break };
        let line = &text[range.start + nl + 1..end];
        if !line[prefix_len(line, kinds.iter().copied())..].trim().is_empty() {
            break;
        }
        end = range.start + text[range.start..range.start + nl].trim_end().len();
    }
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

fn segment(text: &str, events: &[(Event<'static>, Range<usize>)]) -> (Vec<Block>, Vec<Container>) {
    let mut s = Segmenter { text, events, blocks: Vec::new(), containers: Vec::new() };
    s.children(0, events.len(), &[]);
    let mut blocks = s.blocks;
    blocks.sort_by_key(|b| b.range.start);
    (fill_gaps(text, blocks), s.containers)
}

/// Turns the parser's events into blocks and containers.
struct Segmenter<'a> {
    text: &'a str,
    events: &'a [(Event<'static>, Range<usize>)],
    blocks: Vec<Block>,
    containers: Vec<Container>,
}

impl Segmenter<'_> {
    /// The elements among the events `from..to` (siblings), inside the containers `outer`.
    fn children(&mut self, from: usize, to: usize, outer: &[usize]) {
        let mut i = from;
        while i < to {
            let (event, range) = &self.events[i];
            match event {
                Event::Start(tag) => {
                    let end = matching_end(self.events, i);
                    match tag {
                        Tag::List(start) => self.list(i, end, *start, 0, outer, range.start),
                        Tag::BlockQuote(_) => self.quote(i, end, outer),
                        _ => self.leaf(i, end, outer),
                    }
                    i = end + 1;
                }
                Event::Rule => {
                    self.leaf(i, i, outer);
                    i += 1;
                }
                _ => {
                    // Top-level HTML outside an HTML block does not occur; skip anything else.
                    i += 1;
                }
            }
        }
    }

    fn kinds(&self, outer: &[usize]) -> Vec<ContainerKind> {
        outer.iter().map(|&c| self.containers[c].kind).collect()
    }

    /// The start of a block beginning at `offset`: its line's start when
    /// only indentation or container syntax precedes it there.
    fn start(&self, offset: usize, outer: &[usize]) -> usize {
        line_start_if_indented(self.text, offset, !outer.is_empty())
    }

    /// A block for the element started at event `start` (ended at `end`; a
    /// rule when they are the same).
    fn leaf(&mut self, start: usize, end: usize, outer: &[usize]) {
        let (event, range) = &self.events[start];
        let (kind, events) = match event {
            Event::Start(tag) => (block_kind(tag, &self.events[start + 1..end], self.text, range), start + 1..end),
            _ => (BlockKind::ThematicBreak, start..start),
        };
        let range = trim_end(self.text, self.start(range.start, outer)..range.end, &self.kinds(outer));
        self.blocks.push(Block { range, kind, events, outer: outer.to_vec() });
    }

    /// A quote: a container for its blocks, or an empty `Quote` block.
    fn quote(&mut self, start: usize, end: usize, outer: &[usize]) {
        let range = self.start(self.events[start].1.start, outer)..self.events[start].1.end;
        let range = trim_end(self.text, range, &self.kinds(outer));
        let empty = !self.events[start + 1..end].iter().any(|(e, _)| matches!(e, Event::Start(_) | Event::Rule));
        if empty {
            self.blocks.push(Block { range, kind: BlockKind::Quote, events: start + 1..end, outer: outer.to_vec() });
            return;
        }
        self.containers.push(Container { kind: ContainerKind::Quote, range });
        let mut inner = outer.to_vec();
        inner.push(self.containers.len() - 1);
        self.children(start + 1, end, &inner);
    }

    /// Adds a block for every item of the list starting at event `start` (and
    /// ending at `end`), and recursively for its nested lists and for the
    /// blocks of its items after their first one. `top`: where the list at
    /// depth 0 starts.
    #[allow(clippy::too_many_arguments)]
    fn list(&mut self, start: usize, end: usize, first_number: Option<u64>, depth: usize, outer: &[usize], top: usize) {
        let (text, events) = (self.text, self.events);
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
            let item_start = self.start(item_range.start, outer);
            let item = ListItem { depth, number, task, loose, subtree_end: item_range.end, list: top };
            // The item's block: its first block (or a tight item's text), up
            // to its first nested list or other block.
            let mut j = i + 1;
            let mut seen = false;
            while j < item_end {
                let e = &events[j].0;
                if matches!(e, Event::Start(Tag::List(_))) || (seen && is_block(e)) {
                    break;
                }
                seen |= !matches!(e, Event::TaskListMarker(_));
                j = if matches!(e, Event::Start(_)) { matching_end(events, j) } else { j } + 1;
            }
            let own_end =
                if j < item_end { line_start_if_indented(text, events[j].1.start, true) } else { item_range.end };
            let range = trim_end(text, item_start..own_end.max(item_start), &self.kinds(outer));
            let kind = BlockKind::ListItem(item);
            self.blocks.push(Block { range, kind, events: i + 1..j, outer: outer.to_vec() });
            // The rest: nested lists as items, any other block a block of its own inside the item.
            let mut container: Option<usize> = None;
            while j < item_end {
                let e = &events[j].0;
                let child_end = if matches!(e, Event::Start(_)) { matching_end(events, j) } else { j };
                if let Event::Start(Tag::List(n)) = e {
                    self.list(j, child_end, *n, depth + 1, outer, top);
                    j = child_end + 1;
                    continue;
                }
                if matches!(e, Event::TaskListMarker(_)) {
                    j += 1;
                    continue;
                }
                let c = *container.get_or_insert_with(|| {
                    let line_start = text[..item_start].rfind('\n').map_or(0, |p| p + 1);
                    let line = &text[line_start..text[line_start..].find('\n').map_or(text.len(), |e| line_start + e)];
                    let prefix = prefix_len(line, outer.iter().map(|&c| self.containers[c].kind));
                    // The content column: where its first content starts on its first line.
                    let first = events[i + 1..item_end].first().map_or(item_range.end, |(_, r)| r.start);
                    let indent = first.saturating_sub(line_start + prefix).min(line.len() - prefix);
                    let range = trim_end(text, item_start..item_range.end, &self.kinds(outer));
                    self.containers.push(Container { kind: ContainerKind::Item { depth, indent }, range });
                    self.containers.len() - 1
                });
                let mut inner = outer.to_vec();
                inner.push(c);
                if matches!(e, Event::Start(Tag::BlockQuote(_))) {
                    self.quote(j, child_end, &inner);
                } else if is_block(e) {
                    self.leaf(j, child_end, &inner);
                } else {
                    // Text of a tight item after a nested list: a paragraph of its own.
                    let mut k = j;
                    while k < item_end && !is_block(&events[k].0) && !matches!(events[k].0, Event::End(_)) {
                        k = if matches!(events[k].0, Event::Start(_)) { matching_end(events, k) } else { k } + 1;
                    }
                    let start = line_start_if_indented(text, events[j].1.start, true);
                    let range = trim_end(text, start..events[k - 1].1.end, &self.kinds(&inner));
                    self.blocks.push(Block { range, kind: BlockKind::Paragraph, events: j..k, outer: inner });
                    j = k;
                    continue;
                }
                j = child_end + 1;
            }
            i = item_end + 1;
        }
    }
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

/// Adds an `Other` block for each run of non-blank lines no block covers
/// (lines holding only the `>` of quotes are blank).
fn fill_gaps(text: &str, blocks: Vec<Block>) -> Vec<Block> {
    let mut out = Vec::with_capacity(blocks.len());
    let mut pos = 0;
    let gap_blocks = |from: usize, to: usize, out: &mut Vec<Block>| {
        // Non-blank lines, grouped into runs separated by blank lines.
        let mut run: Option<Range<usize>> = None;
        let mut line_start = from;
        for line in text[from..to].split_inclusive('\n') {
            let line_end = line_start + line.len();
            if is_blank(line, true) {
                if let Some(r) = run.take() {
                    out.push(Block {
                        range: trim_end(text, r, &[]),
                        kind: BlockKind::Other,
                        events: 0..0,
                        outer: Vec::new(),
                    });
                }
            } else {
                run = Some(run.map_or(line_start..line_end, |r| r.start..line_end));
            }
            line_start = line_end;
        }
        if let Some(r) = run {
            out.push(Block { range: trim_end(text, r, &[]), kind: BlockKind::Other, events: 0..0, outer: Vec::new() });
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
            ("> q1", BlockKind::Paragraph),
            ("> > q2", BlockKind::Paragraph),
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
    fn content_after_a_nested_list_is_a_block_in_the_item() {
        let b = nested("- a\n\n  - b\n\n  more\n- c\n");
        assert_eq!(b.len(), 4, "{b:#?}");
        assert!(item(0)(&b[0].1) && b[0].0 == "- a");
        assert!(item(1)(&b[1].1) && b[1].0 == "  - b");
        assert_eq!((b[2].0.as_str(), &b[2].1), ("  more", &BlockKind::Paragraph));
        assert_eq!(b[2].2, [ContainerKind::Item { depth: 0, indent: 2 }]);
        assert!(item(0)(&b[3].1) && b[3].0 == "- c");
    }

    /// The blocks of `text`: source, kind and the kinds of the containers around it.
    fn nested(text: &str) -> Vec<(String, BlockKind, Vec<ContainerKind>)> {
        let doc = Document::parse(text);
        (0..doc.blocks.len())
            .map(|b| (text[doc.blocks[b].range.clone()].to_string(), doc.blocks[b].kind.clone(), doc.outer_kinds(b)))
            .collect()
    }

    fn item(depth: usize) -> impl Fn(&BlockKind) -> bool {
        move |k| matches!(k, BlockKind::ListItem(i) if i.depth == depth)
    }

    /// The kind and source of every block of `text`.
    fn item_flags(text: &str) -> Vec<(String, bool)> {
        let doc = Document::parse(text);
        doc.blocks
            .iter()
            .map(|b| (text[b.range.clone()].to_string(), matches!(b.kind, BlockKind::ListItem(_))))
            .collect()
    }

    #[test]
    fn an_empty_item_nested_under_a_paragraph_is_an_item() {
        for marker in ["*", "-", "+", "1."] {
            let pad = " ".repeat(marker.len() + 1);
            let text = format!("{marker} aaa\n{pad}{marker} bbb\n{pad}{pad}{marker} \n");
            let doc = Document::parse(&text);
            let depths: Vec<usize> = doc
                .blocks
                .iter()
                .filter_map(|b| match &b.kind {
                    BlockKind::ListItem(i) => Some(i.depth),
                    _ => None,
                })
                .collect();
            assert_eq!(depths, vec![0, 1, 2], "{text:?}");
            assert_eq!(doc.text, text);
            // The page's text has no letter that stands in for the empty item.
            assert_eq!(plain_text(doc.block_events(2)), "", "{text:?}");
        }
        // An item of a loose list, and one after other lines.
        let doc = Document::parse("- a\n\n  - b\n    - \n\n- c\n");
        assert_eq!(doc.blocks.iter().filter(|b| matches!(b.kind, BlockKind::ListItem(_))).count(), 4);
    }

    #[test]
    fn other_lone_markers_stay_what_they_were() {
        // A setext underline, a marker not nested under an item, a marker after a blank line.
        assert_eq!(item_flags("text\n-\n"), vec![("text\n-".into(), false)]);
        assert_eq!(item_flags("text\n  -\n"), vec![("text\n  -".into(), false)]);
        assert_eq!(item_flags("text\n*\n"), vec![("text\n*".into(), false)]);
        assert_eq!(item_flags("- a\n *\n"), vec![("- a".into(), true), (" *".into(), true)]);
    }

    #[test]
    fn blocks_in_quotes_are_blocks_of_their_own() {
        use ContainerKind::Quote;
        let b = nested("> # H\n> - a\n>   - b\n>\n> para\n> > inner\n\n>\n");
        assert_eq!(b.len(), 6, "{b:#?}");
        assert_eq!((b[0].0.as_str(), &b[0].1, &b[0].2[..]), ("> # H", &BlockKind::Heading(1), &[Quote][..]));
        assert_eq!(b[1].0, "> - a");
        assert!(item(0)(&b[1].1) && b[1].2 == [Quote]);
        assert_eq!(b[2].0, ">   - b");
        assert!(item(1)(&b[2].1) && b[2].2 == [Quote]);
        assert_eq!((b[3].0.as_str(), &b[3].1, &b[3].2[..]), ("> para", &BlockKind::Paragraph, &[Quote][..]));
        assert_eq!((b[4].0.as_str(), &b[4].1, &b[4].2[..]), ("> > inner", &BlockKind::Paragraph, &[Quote, Quote][..]));
        // An empty quote is a block.
        assert_eq!((b[5].0.as_str(), &b[5].1, b[5].2.is_empty()), (">", &BlockKind::Quote, true));
        // Items of a list in a quote share their list.
        let doc = Document::parse("> - a\n> - b\n");
        let lists: Vec<usize> = doc
            .blocks
            .iter()
            .filter_map(|b| match &b.kind {
                BlockKind::ListItem(i) => Some(i.list),
                _ => None,
            })
            .collect();
        assert_eq!(lists, vec![2, 2]);
    }

    #[test]
    fn blocks_of_a_list_item_after_its_first() {
        use ContainerKind::{Item, Quote};
        let b = nested("- a\n\n  ```\n  code\n  ```\n\n  more\n\n  > q\n- b\n");
        assert_eq!(b.len(), 5, "{b:#?}");
        assert_eq!(b[0].0, "- a");
        assert!(item(0)(&b[0].1) && b[0].2.is_empty());
        assert_eq!(b[1].0, "  ```\n  code\n  ```");
        assert!(matches!(b[1].1, BlockKind::Code { .. }));
        assert_eq!(b[1].2, [Item { depth: 0, indent: 2 }]);
        assert_eq!((b[2].0.as_str(), &b[2].1), ("  more", &BlockKind::Paragraph));
        assert_eq!(b[2].2, [Item { depth: 0, indent: 2 }]);
        assert_eq!((b[3].0.as_str(), &b[3].1), ("  > q", &BlockKind::Paragraph));
        assert_eq!(b[3].2, [Item { depth: 0, indent: 2 }, Quote]);
        assert_eq!(b[4].0, "- b");
        // The first block is the item's, whatever it is; a later paragraph is one of its own.
        let b = nested("- ```\n  x\n  ```\n\n- p1\n\n  p2\n");
        assert_eq!(b.len(), 3, "{b:#?}");
        assert_eq!(b[1].0, "- p1");
        assert_eq!((b[2].0.as_str(), &b[2].2[..]), ("  p2", &[Item { depth: 0, indent: 2 }][..]));
        // A list in a quote in an item: depth counts within the quote.
        let b = nested("1. a\n\n   > - x\n");
        assert_eq!(b[1].0, "   > - x");
        assert!(item(0)(&b[1].1));
        assert_eq!(b[1].2, [Item { depth: 0, indent: 3 }, Quote]);
        // A tight item's text after a nested list.
        let b = nested("- a\n  - b\n  c\n");
        assert_eq!(b.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), vec!["- a", "  - b\n  c"]);
    }

    #[test]
    fn container_prefixes() {
        use ContainerKind::{Item, Quote};
        assert_eq!(prefix_len("> a", [Quote]), 2);
        assert_eq!(prefix_len(">a", [Quote]), 1);
        assert_eq!(prefix_len("  > > a", [Quote, Quote]), 6);
        assert_eq!(prefix_len("lazy", [Quote]), 0);
        assert_eq!(prefix_len(">     code", [Quote, Item { depth: 0, indent: 2 }]), 4);
        assert_eq!(prefix_len("> ", [Quote, Item { depth: 0, indent: 2 }]), 2);
    }

    #[test]
    fn a_masked_byte_is_not_syntax() {
        let text = "a\n\n```\n\nb\n\nc\n";
        assert_eq!(Document::parse(text).blocks.len(), 2, "the fence takes the rest");
        let fence = text.find('`').unwrap();
        let doc = Document::parse_masked(text, Some(fence));
        let k: Vec<(&str, &BlockKind)> = doc.blocks.iter().map(|b| (&text[b.range.clone()], &b.kind)).collect();
        assert_eq!(
            k,
            vec![
                ("a", &BlockKind::Paragraph),
                ("```", &BlockKind::Paragraph),
                ("b", &BlockKind::Paragraph),
                ("c", &BlockKind::Paragraph)
            ]
        );
        assert_eq!(plain_text(doc.block_events(1)), "```");
        assert_eq!(doc.text, text);
        // A heading marker.
        let doc = Document::parse_masked("#", Some(0));
        assert_eq!((doc.blocks[0].kind.clone(), plain_text(doc.block_events(0))), (BlockKind::Paragraph, "#".into()));
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
