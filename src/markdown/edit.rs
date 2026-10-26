//! The editing rules of the page editor that go beyond inserting characters:
//! what Enter, Shift+Enter, Backspace, Delete and Tab do depending on the
//! block, as pure functions from the page text and the cursor to the new
//! text and cursor.
//!
//! The cursor is in the *active region*: the source the editor shows, which is
//! a block (or a few, while text is pasted or typed). A new empty paragraph
//! has no source until something is typed into it; it is `Pending`.

use super::parse::{BlockKind, ContainerKind, Document, prefix_len};
use std::ops::Range;

/// A selection in the page text: `anchor` stays, `head` is the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sel {
    pub anchor: usize,
    pub head: usize,
}

impl Sel {
    pub fn caret(at: usize) -> Sel {
        Sel { anchor: at, head: at }
    }

    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

/// An empty paragraph the cursor is in, which gets its source once typed
/// into: `prefix`, the typed text and `suffix` are inserted at `at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub at: usize,
    pub prefix: String,
    pub suffix: String,
    /// The containers it is in (quotes, list items), outermost first.
    pub outer: Vec<ContainerKind>,
}

impl Pending {
    /// An empty paragraph after the block ending at `at`, separated from
    /// what follows by a blank line.
    fn after(text: &str, at: usize) -> Pending {
        let rest = &text[at..];
        let newlines = rest.len() - rest.trim_start_matches('\n').len();
        let suffix = if rest.trim().is_empty() { 0 } else { 2usize.saturating_sub(newlines) };
        // Nothing before it (the only item of a list ended): no blank lines.
        let prefix = if text[..at].trim().is_empty() { "" } else { "\n\n" };
        Pending { at, prefix: prefix.into(), suffix: "\n".repeat(suffix), outer: Vec::new() }
    }

    /// An empty paragraph at `at`, between blocks or in an empty page: blank
    /// lines are added on the sides where a block is near.
    pub fn gap(text: &str, at: usize) -> Pending {
        // At the end of a page, before its last line break (which stays last).
        let at = if at == text.len() && text.ends_with('\n') && !text.trim().is_empty() {
            text.trim_end_matches('\n').len()
        } else {
            at
        };
        let before = &text[..at];
        let after = &text[at..];
        let newlines_before = before.len() - before.trim_end_matches('\n').len();
        let newlines_after = after.len() - after.trim_start_matches('\n').len();
        let prefix = if before.trim().is_empty() { 0 } else { 2usize.saturating_sub(newlines_before) };
        let suffix = if after.trim().is_empty() { 0 } else { 2usize.saturating_sub(newlines_after) };
        Pending { at, prefix: "\n".repeat(prefix), suffix: "\n".repeat(suffix), outer: Vec::new() }
    }

    fn before(at: usize) -> Pending {
        Pending { at, prefix: String::new(), suffix: "\n\n".into(), outer: Vec::new() }
    }

    /// `typed` as it goes into the paragraph: its line breaks followed by
    /// the prefixes of the containers the paragraph is in.
    pub fn prefixed(&self, typed: &str) -> String {
        if self.outer.is_empty() {
            return typed.to_string();
        }
        typed
            .split('\n')
            .enumerate()
            .map(
                |(i, line)| {
                    if i == 0 { line.to_string() } else { container_prefix(&self.outer, line.trim().is_empty()) + line }
                },
            )
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The text with `typed` put in the paragraph, and where `typed` starts.
    pub fn materialize(&self, text: &str, typed: &str) -> (String, usize) {
        let typed = self.prefixed(typed);
        let mut out = String::with_capacity(text.len() + typed.len() + 4);
        out.push_str(&text[..self.at]);
        out.push_str(&self.prefix);
        let start = out.len();
        out.push_str(&typed);
        out.push_str(&self.suffix);
        out.push_str(&text[self.at..]);
        (out, start)
    }
}

/// The outcome of an editing rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edited {
    pub text: String,
    pub sel: Sel,
    /// The cursor is in a new empty paragraph.
    pub pending: Option<Pending>,
}

impl Edited {
    fn new(text: String, at: usize) -> Edited {
        Edited { text, sel: Sel::caret(at), pending: None }
    }

    fn pending(text: String, pending: Pending) -> Edited {
        let at = pending.at;
        Edited { text, sel: Sel::caret(at), pending: Some(pending) }
    }
}

/// The text with the blank lines around `at` (left by a block whose text was
/// all deleted) reduced to one blank line between blocks, or none at the
/// start and end of the page; and where `at` went.
pub fn collapse_blank_lines(text: &str, at: usize) -> (String, usize) {
    let start = at - (text[..at].len() - text[..at].trim_end_matches(['\n', ' ', '\t']).len());
    let end = at + (text[at..].len() - text[at..].trim_start_matches(['\n', ' ', '\t']).len());
    let with = if start == 0 || end == text.len() {
        if end == text.len() && start > 0 && text.ends_with('\n') { "\n" } else { "" }
    } else {
        "\n\n"
    };
    (splice(text, start..end, with), start)
}

/// `text` with `range` replaced by `with`.
pub fn splice(text: &str, range: Range<usize>, with: &str) -> String {
    let mut out = String::with_capacity(text.len() + with.len());
    out.push_str(&text[..range.start]);
    out.push_str(with);
    out.push_str(&text[range.end..]);
    out
}

/// The lines of a container without the prefixes (`>`, indentation) of the
/// containers around a block in it: the text the editing rules see for a
/// block inside quotes and list items, which they treat as a page.
struct Inner {
    /// The page's whole lines these are.
    span: Range<usize>,
    /// For each line: where it starts in the page and the length of its prefix.
    lines: Vec<(usize, usize)>,
    text: String,
    /// The containers, outermost first.
    kinds: Vec<ContainerKind>,
}

impl Inner {
    /// The lines of the innermost container around the block at `offset`
    /// (for a list item, its lines after the first); None for a block in no
    /// container.
    fn of(text: &str, doc: &Document, offset: usize) -> Option<Inner> {
        let block = doc.blocks.iter().position(|b| b.range.start <= offset && offset <= b.range.end)?;
        let &innermost = doc.blocks[block].outer.last()?;
        Some(Inner::within(text, doc, &doc.blocks[block].outer[..], innermost))
    }

    /// The lines of container `innermost`, inside the containers `outer` (it included).
    fn within(text: &str, doc: &Document, outer: &[usize], innermost: usize) -> Inner {
        let c = &doc.containers[innermost];
        let start = match c.kind {
            ContainerKind::Quote => line_range(text, c.range.start).start,
            ContainerKind::Item { .. } => (line_range(text, c.range.start).end + 1).min(c.range.end),
        };
        let span = start..line_range(text, c.range.end).end;
        let kinds: Vec<ContainerKind> = outer.iter().map(|&i| doc.containers[i].kind).collect();
        let mut lines = Vec::new();
        let mut inner = String::with_capacity(span.len());
        let mut pos = span.start;
        for line in text[span.clone()].split('\n') {
            let prefix = prefix_len(line, kinds.iter().copied());
            if !lines.is_empty() {
                inner.push('\n');
            }
            inner.push_str(&line[prefix..]);
            lines.push((pos, prefix));
            pos += line.len() + 1;
        }
        Inner { span, lines, text: inner, kinds }
    }

    /// The offset in the lines of a page offset (one in a prefix: its line's content start).
    fn to_inner(&self, offset: usize) -> usize {
        let offset = offset.clamp(self.span.start, self.span.end);
        let k = self.lines.partition_point(|&(start, _)| start <= offset).saturating_sub(1);
        let inner_start: usize = self.lines[..k].iter().zip(&self.lines[1..]).map(|(a, b)| b.0 - a.0 - a.1).sum();
        let (start, prefix) = self.lines[k];
        inner_start + offset.saturating_sub(start + prefix)
    }

    fn prefix(&self, blank: bool) -> String {
        container_prefix(&self.kinds, blank)
    }

    /// The page with the lines replaced by `new` (a rule's outcome on them):
    /// lines the rule left as they were keep their prefix, the others get the
    /// containers' prefix (that of a line with text on the line holding the
    /// offset `open`, where text is going to be typed); and the page offset
    /// of each offset of `new`.
    fn put_back(&self, page: &str, new: &str, open: Option<usize>) -> (String, impl Fn(usize) -> usize + use<>) {
        let old = &self.text;
        let same_start = old.bytes().zip(new.bytes()).take_while(|(a, b)| a == b).count();
        let same_end = old.bytes().rev().zip(new.bytes().rev()).take_while(|(a, b)| a == b).count();
        let same_end = same_end.min(old.len() - same_start).min(new.len() - same_start);
        let old_starts: Vec<usize> = std::iter::once(0).chain(old.match_indices('\n').map(|(i, _)| i + 1)).collect();
        let mut out = String::with_capacity(page.len() + new.len());
        out.push_str(&page[..self.span.start]);
        // For each line of `new`: where its content starts in the page.
        let mut starts: Vec<(usize, usize)> = Vec::new();
        let mut at = 0;
        for line in new.split('\n') {
            if at > 0 {
                out.push('\n');
            }
            let old_line = if at < same_start {
                old_starts.binary_search(&at).ok()
            } else if at >= new.len() - same_end {
                old_starts.binary_search(&(at + old.len() - new.len())).ok()
            } else {
                None
            };
            let typed_into = open.is_some_and(|o| at <= o && o <= at + line.len());
            let prefix = match old_line {
                // A blank line to type on gets a whole prefix, whatever it had.
                _ if typed_into && line.trim().is_empty() => self.prefix(false),
                Some(k) => {
                    let (start, len) = self.lines[k];
                    page[start..start + len].to_string()
                }
                None => self.prefix(line.trim().is_empty()),
            };
            out.push_str(&prefix);
            starts.push((at, out.len()));
            out.push_str(line);
            at += line.len() + 1;
        }
        out.push_str(&page[self.span.end..]);
        let map = move |o: usize| {
            let k = starts.partition_point(|&(s, _)| s <= o).saturating_sub(1);
            starts[k].1 + (o - starts[k].0)
        };
        (out, map)
    }

    /// A rule's outcome on the lines as the page's.
    fn edited(&self, page: &str, e: Edited) -> Edited {
        let open = match &e.pending {
            Some(p) => (!p.prefix.contains('\n')).then_some(p.at),
            None => Some(e.sel.head),
        };
        let (text, map) = self.put_back(page, &e.text, open);
        let sel = Sel { anchor: map(e.sel.anchor), head: map(e.sel.head) };
        let pending = e.pending.map(|p| {
            let at = map(p.at);
            // The new paragraph's lines get the prefixes as well.
            let ends_blank = text[at..].starts_with('\n') || at == text.len();
            Pending {
                at,
                prefix: self.prefixed(&p.prefix, false),
                suffix: self.prefixed(&p.suffix, ends_blank),
                outer: self.kinds.iter().copied().chain(p.outer).collect(),
            }
        });
        Edited { text, sel, pending }
    }

    /// `s` with the prefix after each of its line breaks; `last_blank`:
    /// whether the line its last line break starts stays blank.
    fn prefixed(&self, s: &str, last_blank: bool) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find('\n') {
            out.push_str(&rest[..=i]);
            rest = &rest[i + 1..];
            let blank = if rest.contains('\n') { rest.starts_with('\n') } else { rest.is_empty() && last_blank };
            out.push_str(&self.prefix(blank));
        }
        out.push_str(rest);
        out
    }
}

/// The prefix a new line inside the containers `kinds` (outermost first)
/// gets: their prefixes, `>` alone on a blank line.
pub fn container_prefix(kinds: &[ContainerKind], blank: bool) -> String {
    let n = kinds.len();
    let p: String = kinds.iter().enumerate().map(|(i, k)| k.line_prefix(blank && i + 1 == n)).collect();
    if blank { p.trim_end().to_string() } else { p }
}

/// `nested` for a range of blocks: inside their containers when they all
/// share them, else as it is.
pub fn nested_blocks(
    text: &str,
    doc: &Document,
    range: Range<usize>,
    sel: Sel,
    rule: impl FnOnce(&str, &Document, Range<usize>, Sel) -> Option<Edited>,
) -> Option<Edited> {
    let mut outers = doc
        .blocks
        .iter()
        .filter(|b| b.range.end >= range.start && b.range.start <= range.end.max(range.start))
        .map(|b| &b.outer);
    let first = outers.next();
    if first.is_some_and(|f| !f.is_empty() && outers.all(|o| o == f)) {
        let inner =
            Inner::of(text, doc, range.start.max(doc.blocks.iter().find(|b| b.range.end >= range.start)?.range.start))?;
        let inner_doc = Document::parse(&inner.text);
        let r = inner.to_inner(range.start)..inner.to_inner(range.end);
        let s = Sel { anchor: inner.to_inner(sel.anchor), head: inner.to_inner(sel.head) };
        let e = rule(&inner.text, &inner_doc, r, s)?;
        return Some(inner.edited(text, e));
    }
    rule(text, doc, range, sel)
}

/// Applies the editing `rule` to the block at `region` inside the containers
/// around it (quotes, list items): to the lines of the innermost one without
/// their prefixes, as to a page; the lines it changes get the prefixes back.
/// A block in no container gets the rule as it is.
pub fn nested(
    text: &str,
    doc: &Document,
    region: Range<usize>,
    sel: Sel,
    rule: impl FnOnce(&str, &Document, Range<usize>, Sel) -> Option<Edited>,
) -> Option<Edited> {
    let Some(inner) = Inner::of(text, doc, region.start) else { return rule(text, doc, region, sel) };
    let inner_doc = Document::parse(&inner.text);
    let r = inner.to_inner(region.start)..inner.to_inner(region.end);
    let s = Sel { anchor: inner.to_inner(sel.anchor), head: inner.to_inner(sel.head) };
    let e = rule(&inner.text, &inner_doc, r, s)?;
    Some(inner.edited(text, e))
}

/// Backspace (see `backspace`) inside the containers around the block: at
/// the start of the first block of a quote, the block leaves the quote; of
/// a list item, it joins the item's text as the block before it.
pub fn backspace_nested(text: &str, doc: &Document, region: Range<usize>, c: usize) -> Option<Edited> {
    let Some(inner) = Inner::of(text, doc, region.start) else { return backspace(text, doc, region, c) };
    let inner_doc = Document::parse(&inner.text);
    let r = inner.to_inner(region.start)..inner.to_inner(region.end);
    let ic = inner.to_inner(c);
    if let Some(e) = backspace(&inner.text, &inner_doc, r.clone(), ic) {
        return Some(inner.edited(text, e));
    }
    if ic != r.start {
        return None;
    }
    match inner.kinds.last() {
        Some(ContainerKind::Item { .. }) => backspace(text, doc, region.clone(), region.start),
        _ => leave_container(text, doc, region),
    }
}

/// The block at `region` out of the innermost container around it: that
/// container's prefix off its lines, and off a blank line after it.
fn leave_container(text: &str, doc: &Document, region: Range<usize>) -> Option<Edited> {
    let block = doc.blocks.iter().position(|b| b.range.start == region.start)?;
    let kinds = doc.outer_kinds(block);
    if kinds.is_empty() {
        return None;
    }
    let outer = &kinds[..kinds.len() - 1];
    let lines = line_range(text, region.start).start..line_range(text, region.end).end;
    // Its lines, and the blank one after it.
    let next = line_range(text, (lines.end + 1).min(text.len()));
    let next_blank = lines.end < text.len() && {
        let line = &text[next.clone()];
        line[prefix_len(line, kinds.iter().copied())..].trim().is_empty()
    };
    let end = if next_blank { next.end } else { lines.end };
    let content = content_start(text, region.start);
    let mut out = String::new();
    let mut removed_before = 0;
    let mut pos = lines.start;
    for (i, line) in text[lines.start..end].split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let keep = prefix_len(line, outer.iter().copied());
        let drop = prefix_len(line, kinds.iter().copied()) - keep;
        let rest = &line[keep + drop..];
        if rest.trim().is_empty() {
            out.push_str(line[..keep].trim_end());
        } else {
            out.push_str(&line[..keep]);
            out.push_str(rest);
        }
        if pos + keep + drop <= content {
            removed_before += drop;
        }
        pos += line.len() + 1;
    }
    let t = splice(text, lines.start..end, &out);
    Some(Edited::new(t, content - removed_before))
}

/// A new empty paragraph at `at`, a position between blocks, inside the
/// containers around `at` (their first `depth` only, when given).
pub fn gap_pending(text: &str, doc: &Document, at: usize, depth: Option<usize>) -> Pending {
    // The containers around it, outermost first (as they were found): those
    // whose lines it is on (a list item's after its first).
    let mut around: Vec<usize> = (0..doc.containers.len())
        .filter(|&c| {
            let r = &doc.containers[c].range;
            let first_line_end = line_range(text, r.start).end;
            let start = match doc.containers[c].kind {
                ContainerKind::Quote => r.start,
                ContainerKind::Item { .. } => first_line_end + 1,
            };
            start <= at && at <= line_range(text, r.end).end
        })
        .collect();
    if let Some(d) = depth {
        around.truncate(d);
    }
    let Some(&innermost) = around.last() else { return Pending::gap(text, at) };
    let inner = Inner::within(text, doc, &around, innermost);
    let p = Pending::gap(&inner.text, inner.to_inner(at));
    // The text stays as it is: a prefix the line it goes on lacks comes with it.
    let (_, map) = inner.put_back(text, &inner.text, None);
    let at = map(p.at);
    let mut prefix = inner.prefixed(&p.prefix, false);
    if !p.prefix.contains('\n') {
        let present = &text[line_range(text, at).start..at];
        if let Some(missing) = inner.prefix(false).strip_prefix(present) {
            prefix = format!("{missing}{prefix}");
        }
    }
    let ends_blank = text[at..].starts_with('\n') || at == text.len();
    Pending { at, prefix, suffix: inner.prefixed(&p.suffix, ends_blank), outer: inner.kinds }
}

/// The marker at the start of a block's first line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    /// Leading spaces.
    pub indent: usize,
    pub kind: MarkerKind,
    /// Where the marker ends (the content starts), from the line start.
    pub end: usize,
    /// For a task item, the range of `[ ] ` from the line start.
    pub task: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerKind {
    Heading(u8),
    Bullet(char),
    /// The number and its delimiter (`.` or `)`).
    Ordered(u64, char),
    /// Quote markers, possibly nested (`> > `).
    Quote,
}

/// The block marker a line starts with, if any.
pub fn line_marker(line: &str) -> Option<Marker> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    let bytes = rest.as_bytes();
    let first = *bytes.first()?;
    // At least one space after the marker, or the end of the line.
    let spaced = |len: usize| rest.len() == len || rest[len..].starts_with([' ', '\t']);
    let with_space = |len: usize| {
        let spaces = rest[len..].len() - rest[len..].trim_start_matches([' ', '\t']).len();
        indent + len + spaces.min(4)
    };
    match first {
        b'#' => {
            let n = rest.len() - rest.trim_start_matches('#').len();
            (n <= 6 && spaced(n)).then(|| Marker {
                indent,
                kind: MarkerKind::Heading(n as u8),
                end: with_space(n),
                task: None,
            })
        }
        b'-' | b'*' | b'+' if spaced(1) => {
            let end = with_space(1);
            let task = task_marker(line, end);
            let end = task.as_ref().map_or(end, |t| t.end);
            Some(Marker { indent, kind: MarkerKind::Bullet(first as char), end, task })
        }
        b'0'..=b'9' => {
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let delim = *bytes.get(digits)?;
            if digits > 9 || !matches!(delim, b'.' | b')') || !spaced(digits + 1) {
                return None;
            }
            let n = rest[..digits].parse().ok()?;
            let end = with_space(digits + 1);
            let task = task_marker(line, end);
            let end = task.as_ref().map_or(end, |t| t.end);
            Some(Marker { indent, kind: MarkerKind::Ordered(n, delim as char), end, task })
        }
        b'>' => {
            let mut end = indent;
            let b = line.as_bytes();
            while end < b.len() && (b[end] == b'>' || b[end] == b' ') {
                end += 1;
            }
            Some(Marker { indent, kind: MarkerKind::Quote, end, task: None })
        }
        _ => None,
    }
}

/// `[ ] ` or `[x] ` at `at` in a list item's first line.
fn task_marker(line: &str, at: usize) -> Option<Range<usize>> {
    let rest = &line[at..];
    let ok = rest.len() >= 3
        && rest.as_bytes()[0] == b'['
        && rest.as_bytes()[2] == b']'
        && matches!(rest.as_bytes()[1], b' ' | b'x' | b'X');
    if !ok || !(rest.len() == 3 || rest[3..].starts_with(' ')) {
        return None;
    }
    let end = at + 3 + usize::from(rest.len() > 3);
    Some(at..end)
}

/// The range of the line holding `offset`, without its line break.
pub fn line_range(text: &str, offset: usize) -> Range<usize> {
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let end = text[offset..].find('\n').map_or(text.len(), |i| offset + i);
    start..end
}

/// The kind of the active region's block: the page's block starting there,
/// else the region parsed on its own.
fn region_kind(doc: &Document, text: &str, region: &Range<usize>) -> BlockKind {
    if let Some(b) =
        doc.blocks.iter().find(|b| b.range.start == region.start && b.range.end <= region.end.max(b.range.end))
    {
        return b.kind.clone();
    }
    let own = Document::parse(&text[region.clone()]);
    own.blocks.first().map_or(BlockKind::Paragraph, |b| b.kind.clone())
}

/// The region of the block holding `offset`: its range with the spaces
/// that follow it on its last line (`- ` of an empty item keeps its space).
pub fn region_at(doc: &Document, offset: usize) -> Option<Range<usize>> {
    let with_spaces = |r: Range<usize>| {
        let rest = &doc.text[r.end..];
        let spaces = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        r.start..r.end + spaces
    };
    if let Some(b) = doc.block_at(offset) {
        return Some(with_spaces(doc.blocks[b].range.clone()));
    }
    let b = doc.blocks.iter().rev().find(|b| b.range.end <= offset)?;
    let r = with_spaces(b.range.clone());
    (offset <= r.end).then_some(r)
}

/// A line consisting of a code fence opening: its fence characters.
fn fence_opening(line: &str) -> Option<&str> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return None;
    }
    let c = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let n = t.len() - t.trim_start_matches(c).len();
    let info = &t[n..];
    (n >= 3 && !(c == '`' && info.contains('`'))).then(|| &t[..n])
}

/// A table header row: `| a | b |`.
fn is_table_header(line: &str) -> bool {
    let t = line.trim();
    t.len() >= 3 && t.starts_with('|') && t.ends_with('|') && !t.ends_with("\\|")
}

/// The number of cells of a table row.
pub fn row_cells(line: &str) -> usize {
    super::table::split_row(line).len()
}

/// The block before / after the region, in the page's parse.
fn neighbors(doc: &Document, region: &Range<usize>) -> (Option<usize>, Option<usize>) {
    let before = doc.blocks.iter().rposition(|b| b.range.end <= region.start && b.range.start < region.start);
    let after = doc.blocks.iter().position(|b| b.range.start >= region.end && b.range.end > region.end);
    (before, after)
}

/// Where the content of a block's source starts (after its first line's marker).
pub fn content_start(text: &str, block_start: usize) -> usize {
    let line = line_range(text, block_start);
    match line_marker(&text[line.clone()]) {
        Some(m) => line.start + m.end,
        None => block_start + (text[block_start..line.end].len() - text[block_start..line.end].trim_start().len()),
    }
}

/// Enter at the cursor of the active `region`. None when the text editor's
/// own line break is wanted (tables are edited elsewhere).
pub fn enter(text: &str, doc: &Document, region: Range<usize>, sel: Sel) -> Option<Edited> {
    if !sel.is_empty() {
        // Replace the selection, then break there.
        let r = sel.range();
        let cut = splice(text, r.clone(), "");
        let region = region.start..region.end - (r.end.min(region.end) - r.start.max(region.start).min(region.end));
        let cut_doc = Document::parse(&cut);
        return enter(&cut, &cut_doc, region, Sel::caret(r.start)).or_else(|| Some(Edited::new(cut, r.start)));
    }
    let c = sel.head;
    let src = &text[region.clone()];
    let line = line_range(text, c);
    let line_text = &text[line.clone()];
    let at_end = c == region.end;
    let kind = region_kind(doc, text, &region);

    // Syntax completed by Enter on a single line.
    if !src.contains('\n') && at_end {
        if let Some(fence) = fence_opening(src) {
            return Some(Edited::new(splice(text, c..c, &format!("\n\n{fence}")), c + 1));
        }
        if src.trim() == "$$" {
            return Some(Edited::new(splice(text, c..c, "\n\n$$"), c + 1));
        }
        if src.trim() == "---" && text[..region.start].trim().is_empty() {
            return Some(Edited::new(splice(text, c..c, "\n\n---"), c + 1));
        }
        if is_table_header(src) && !matches!(kind, BlockKind::Table) {
            let cells = row_cells(src).max(1);
            let delimiter = format!("|{}", " --- |".repeat(cells));
            let empty = format!("|{}", "  |".repeat(cells));
            let insert = format!("\n{delimiter}\n{empty}");
            let first_cell = c + 1 + delimiter.len() + 1 + 2;
            return Some(Edited::new(splice(text, c..c, &insert), first_cell));
        }
    }

    match kind {
        BlockKind::Table => None,
        BlockKind::Code { .. } | BlockKind::Math | BlockKind::FrontMatter | BlockKind::Html => {
            let indent: String = text[line.start..c].chars().take_while(|c| *c == ' ' || *c == '\t').collect();
            Some(Edited::new(splice(text, c..c, &format!("\n{indent}")), c + 1 + indent.len()))
        }
        BlockKind::ListItem(item) => Some(enter_list_item(text, region, c, item.loose)),
        BlockKind::Quote => Some(enter_quote(text, region, c, line, line_text)),
        BlockKind::ThematicBreak => Some(Edited::pending(text.to_string(), Pending::after(text, region.end))),
        _ => {
            let content = content_start(text, region.start).min(region.end);
            if at_end {
                Some(Edited::pending(text.to_string(), Pending::after(text, region.end)))
            } else if c <= content && c < region.end {
                Some(Edited::pending(text.to_string(), Pending::before(region.start)))
            } else {
                Some(Edited::new(splice(text, c..c, "\n\n"), c + 2))
            }
        }
    }
}

fn enter_list_item(text: &str, region: Range<usize>, c: usize, loose: bool) -> Edited {
    let first = line_range(text, region.start);
    let Some(m) = line_marker(&text[first.clone()]) else { return Edited::new(splice(text, c..c, "\n"), c + 1) };
    let content = first.start + m.end;
    let empty = text[content..region.end].trim().is_empty();
    if empty {
        if m.indent > 0 {
            // Outdent a nested empty item to its parent's level.
            let parent_indent = parent_indent(text, region.start, m.indent);
            let removed = m.indent - parent_indent;
            let t = splice(text, first.start..first.start + removed, "");
            return Edited::new(t, c - removed);
        }
        // End the list: the empty item goes, an empty paragraph follows the list.
        let line_with_break = if first.start > 0 { first.start - 1..region.end } else { first.start..region.end };
        let t = splice(text, line_with_break.clone(), "");
        let pending = Pending::after(&t, line_with_break.start);
        return Edited::pending(t, pending);
    }
    let marker = match &m.kind {
        MarkerKind::Bullet(b) => format!("{b} "),
        MarkerKind::Ordered(n, d) => format!("{}{d} ", n + 1),
        _ => String::new(),
    };
    let task = if m.task.is_some() { "[ ] " } else { "" };
    let indent = " ".repeat(m.indent);
    let separator = if loose { "\n\n" } else { "\n" };
    // Text after the cursor moves into the new item, without leading spaces.
    let rest_start = c + (text[c..region.end].len() - text[c..region.end].trim_start_matches(' ').len());
    let insert = format!("{separator}{indent}{marker}{task}");
    let t = splice(text, c..rest_start, &insert);
    Edited::new(t, c + insert.len())
}

/// The indentation of the list item a nested item at `start` (indented by
/// `indent`) belongs to.
fn parent_indent(text: &str, start: usize, indent: usize) -> usize {
    let mut pos = start;
    while pos > 0 {
        let line = line_range(text, pos - 1);
        if let Some(m) = line_marker(&text[line.clone()])
            && matches!(m.kind, MarkerKind::Bullet(_) | MarkerKind::Ordered(..))
            && m.indent < indent
        {
            return m.indent;
        }
        pos = line.start;
    }
    0
}

fn enter_quote(text: &str, region: Range<usize>, c: usize, line: Range<usize>, line_text: &str) -> Edited {
    let markers: String = line_text.chars().take_while(|c| *c == '>' || *c == ' ').collect();
    if line_text.trim().chars().all(|ch| ch == '>') {
        // An empty quoted line: leave the quote, removing the line (and the
        // empty line before it, that Enter added).
        let mut start = line.start;
        if start > region.start {
            let prev = line_range(text, start - 1);
            if text[prev.clone()].trim().chars().all(|ch| ch == '>') && prev.start > region.start {
                start = prev.start;
            }
        }
        let remove = start.saturating_sub(1).max(region.start)..line.end;
        let t = splice(text, remove.clone(), "");
        let pending = Pending::after(&t, remove.start);
        return Edited::pending(t, pending);
    }
    let quote = markers.trim_end();
    let marker = if markers.ends_with(' ') { markers.clone() } else { format!("{markers} ") };
    let insert = format!("\n{quote}\n{marker}");
    Edited::new(splice(text, c..c, &insert), c + insert.len())
}

/// Shift+Enter: a line break inside the block.
pub fn shift_enter(text: &str, doc: &Document, region: Range<usize>, sel: Sel) -> Option<Edited> {
    let r = sel.range();
    let c = r.start;
    match region_kind(doc, text, &region) {
        BlockKind::Table => None,
        BlockKind::Heading(_) => enter(text, doc, region, sel),
        BlockKind::Code { .. } | BlockKind::Math | BlockKind::FrontMatter | BlockKind::Html => {
            enter(text, doc, region, sel)
        }
        BlockKind::ListItem(_) => {
            let first = line_range(text, region.start);
            let width = line_marker(&text[first.clone()]).map_or(0, |m| m.end - m.task.map_or(0, |t| t.len()));
            let insert = format!("\n{}", " ".repeat(width));
            Some(Edited::new(splice(text, r, &insert), c + insert.len()))
        }
        BlockKind::Quote => {
            let line = line_range(text, c);
            let markers: String = text[line].chars().take_while(|c| *c == '>' || *c == ' ').collect();
            let insert = format!("\n{markers}");
            Some(Edited::new(splice(text, r, &insert), c + insert.len()))
        }
        _ => Some(Edited::new(splice(text, r, "\n"), c + 1)),
    }
}

/// Backspace with nothing selected: removes a block's marker right before
/// the cursor, or joins the block to the one before. None leaves it to the
/// text editor.
pub fn backspace(text: &str, doc: &Document, region: Range<usize>, c: usize) -> Option<Edited> {
    let first = line_range(text, region.start);
    if c > region.start
        && c <= first.end
        && let Some(m) = line_marker(&text[first.clone()])
        && c == first.start + m.end
    {
        // A task item loses its box first, then its bullet.
        let remove = match &m.task {
            Some(task) => first.start + task.start..first.start + task.end,
            None => first.start + m.indent..first.start + m.end,
        };
        return Some(Edited::new(splice(text, remove.clone(), ""), remove.start));
    }
    if c != region.start {
        return None;
    }
    let (before, _) = neighbors(doc, &region);
    let prev = &doc.blocks[before?];
    let kind = region_kind(doc, text, &region);
    if prev.kind.is_text() && kind.is_text() {
        let from = content_start(text, region.start);
        let t = splice(text, prev.range.end..from, "");
        Some(Edited::new(t, prev.range.end))
    } else if prev.kind == BlockKind::ThematicBreak {
        let t = splice(text, prev.range.start..region.start, "");
        Some(Edited::new(t, prev.range.start))
    } else {
        Some(Edited::new(text.to_string(), prev.range.end))
    }
}

/// Delete with nothing selected at the end of the region: joins the next
/// block. None leaves it to the text editor.
pub fn delete(text: &str, doc: &Document, region: Range<usize>, c: usize) -> Option<Edited> {
    if c != region.end {
        return None;
    }
    let (_, after) = neighbors(doc, &region);
    let next = &doc.blocks[after?];
    let kind = region_kind(doc, text, &region);
    if next.kind.is_text() && kind.is_text() {
        let to = content_start(text, next.range.start);
        Some(Edited::new(splice(text, region.end..to, ""), c))
    } else if next.kind == BlockKind::ThematicBreak {
        let end =
            doc.blocks.iter().find(|b| b.range.start > next.range.start).map_or(next.range.end, |b| b.range.start);
        Some(Edited::new(splice(text, region.end..end, "\n\n"), c))
    } else {
        Some(Edited::new(text.to_string(), c))
    }
}

/// Tab (indent) or Shift+Tab (outdent) in the region: list items move a
/// level with their nested items; code lines are indented. None when Tab
/// has nothing to do here (it is swallowed).
pub fn indent(text: &str, doc: &Document, region: Range<usize>, sel: Sel, outdent: bool) -> Option<Edited> {
    match region_kind(doc, text, &region) {
        BlockKind::ListItem(_) => indent_list_item(text, doc, region, sel, outdent),
        BlockKind::Code { .. } | BlockKind::Math | BlockKind::Html | BlockKind::FrontMatter => {
            Some(indent_lines(text, sel, outdent))
        }
        _ => None,
    }
}

const CODE_INDENT: &str = "    ";

/// Indents (or outdents) the lines of the selection, or inserts an
/// indentation at the cursor.
fn indent_lines(text: &str, sel: Sel, outdent: bool) -> Edited {
    let r = sel.range();
    if r.is_empty() && !outdent {
        return Edited::new(splice(text, r.clone(), CODE_INDENT), r.start + CODE_INDENT.len());
    }
    let first = line_range(text, r.start).start;
    let last = line_range(text, r.end).end;
    let mut out = String::new();
    let mut shift_start: isize = 0;
    let mut shift_end: isize = 0;
    let mut pos = first;
    for line in text[first..last].split('\n') {
        let (new, delta) = if outdent {
            let spaces = (line.len() - line.trim_start_matches(' ').len()).min(CODE_INDENT.len());
            (line[spaces..].to_string(), -(spaces as isize))
        } else {
            (format!("{CODE_INDENT}{line}"), CODE_INDENT.len() as isize)
        };
        if pos <= r.start {
            shift_start += delta;
        }
        if pos <= r.end {
            shift_end += delta;
        }
        out.push_str(&new);
        out.push('\n');
        pos += line.len() + 1;
    }
    out.pop();
    let t = splice(text, first..last, &out);
    let s = (r.start as isize + shift_start).max(first as isize) as usize;
    let e = (r.end as isize + shift_end).max(s as isize) as usize;
    Edited { text: t, sel: Sel { anchor: s, head: e }, pending: None }
}

fn indent_list_item(text: &str, doc: &Document, region: Range<usize>, sel: Sel, outdent: bool) -> Option<Edited> {
    let index = doc.blocks.iter().position(|b| b.range.start == region.start)?;
    let BlockKind::ListItem(item) = &doc.blocks[index].kind else { return None };
    let first = line_range(text, region.start);
    let m = line_marker(&text[first.clone()])?;
    // The item with its nested items.
    let end = text[..item.subtree_end].trim_end().len().max(region.end);
    let amount = if outdent {
        if item.depth == 0 {
            return Some(Edited { text: text.to_string(), sel, pending: None });
        }
        m.indent - parent_indent(text, region.start, m.indent)
    } else {
        // Under the previous item of the same list (past the blocks inside items).
        let outer = doc.blocks[index].outer.len();
        let prev = doc.blocks[..index].iter().rev().find_map(|b| match &b.kind {
            _ if b.outer.len() > outer => None,
            BlockKind::ListItem(p) if p.depth <= item.depth => Some((b, p.depth)),
            BlockKind::ListItem(_) => None,
            _ => Some((b, usize::MAX)),
        });
        let Some((prev, depth)) = prev.filter(|(_, d)| *d == item.depth) else {
            return Some(Edited { text: text.to_string(), sel, pending: None });
        };
        let _ = depth;
        let pm = line_marker(&text[line_range(text, prev.range.start)])?;
        pm.end - pm.indent - pm.task.map_or(0, |t| t.len())
    };
    let mut out = String::new();
    let shift = |offset: usize| -> isize {
        // How far `offset` moves: the change of every line start before it.
        let mut delta = 0isize;
        let mut pos = first.start;
        for line in text[first.start..end].split('\n') {
            if pos > offset {
                break;
            }
            let spaces = line.len() - line.trim_start_matches(' ').len();
            if outdent {
                delta -= spaces.min(amount) as isize;
            } else if !line.trim().is_empty() {
                delta += amount as isize;
            }
            pos += line.len() + 1;
        }
        delta
    };
    let (s, h) = (shift(sel.anchor), shift(sel.head));
    for line in text[first.start..end].split('\n') {
        if outdent {
            let spaces = line.len() - line.trim_start_matches(' ').len();
            out.push_str(&line[spaces.min(amount)..]);
        } else if line.trim().is_empty() {
            out.push_str(line);
        } else {
            out.push_str(&" ".repeat(amount));
            out.push_str(line);
        }
        out.push('\n');
    }
    out.pop();
    let t = splice(text, first.start..end, &out);
    let clamp = |o: usize, d: isize| ((o as isize + d).max(first.start as isize)) as usize;
    Some(Edited { text: t, sel: Sel { anchor: clamp(sel.anchor, s), head: clamp(sel.head, h) }, pending: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `text` with `‸` as the cursor; the region is the block holding it.
    fn setup(marked: &str) -> (String, Range<usize>, usize) {
        let (c, len) = match marked.find('‸') {
            Some(c) => (c, '‸'.len_utf8()),
            None => (marked.rfind('|').expect("cursor"), 1),
        };
        let text = format!("{}{}", &marked[..c], &marked[c + len..]);
        let doc = Document::parse(&text);
        let region = region_at(&doc, c).unwrap_or(c..c);
        (text, region, c)
    }

    fn show(e: &Edited) -> String {
        let mut t = e.text.clone();
        match &e.pending {
            Some(p) => {
                let (m, start) = p.materialize(&t, "¶");
                t = m;
                let _ = start;
            }
            None => t.insert(e.sel.head, '‸'),
        }
        t
    }

    fn enter_at(marked: &str) -> String {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        show(&nested(&text, &doc, region, Sel::caret(c), enter).expect("handled"))
    }

    fn shift_enter_at(marked: &str) -> String {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        show(&nested(&text, &doc, region, Sel::caret(c), shift_enter).expect("handled"))
    }

    fn backspace_at(marked: &str) -> Option<String> {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        backspace_nested(&text, &doc, region, c).map(|e| show(&e))
    }

    fn delete_at(marked: &str) -> Option<String> {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        nested(&text, &doc, region, Sel::caret(c), |t, d, r, s| delete(t, d, r, s.head)).map(|e| show(&e))
    }

    fn tab_at(marked: &str, outdent: bool) -> String {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        show(&nested(&text, &doc, region, Sel::caret(c), |t, d, r, s| indent(t, d, r, s, outdent)).expect("handled"))
    }

    #[test]
    fn enter_splits_and_appends_paragraphs() {
        assert_eq!(enter_at("one‸two"), "one\n\n‸two");
        // At the end or the start: an empty paragraph, without source yet.
        assert_eq!(enter_at("one‸\n\nthree"), "one\n\n¶\n\nthree");
        assert_eq!(enter_at("one‸"), "one\n\n¶");
        assert_eq!(enter_at("‸one"), "¶\n\none");
        assert_eq!(enter_at("# Plan‸"), "# Plan\n\n¶");
        assert_eq!(enter_at("# Pl‸an"), "# Pl\n\n‸an");
    }

    #[test]
    fn enter_completes_fences_front_matter_and_tables() {
        assert_eq!(enter_at("```rust‸"), "```rust\n‸\n```");
        assert_eq!(enter_at("text\n\n~~~‸"), "text\n\n~~~\n‸\n~~~");
        assert_eq!(enter_at("$$‸"), "$$\n‸\n$$");
        assert_eq!(enter_at("---‸"), "---\n‸\n---");
        assert_eq!(enter_at("para\n\n---‸"), "para\n\n---\n\n¶");
        let (text, region, c) = setup("| Name | Note |‸");
        let doc = Document::parse(&text);
        let e = enter(&text, &doc, region, Sel::caret(c)).unwrap();
        assert_eq!(e.text, "| Name | Note |\n| --- | --- |\n|  |  |");
        // The cursor in the first body cell.
        assert_eq!(&e.text[..e.sel.head], "| Name | Note |\n| --- | --- |\n| ");
    }

    #[test]
    fn enter_in_code_keeps_the_indentation() {
        assert_eq!(enter_at("```\n    let x;‸\n```"), "```\n    let x;\n    ‸\n```");
    }

    #[test]
    fn enter_continues_lists() {
        assert_eq!(enter_at("- one‸"), "- one\n- ‸");
        assert_eq!(enter_at("* one‸"), "* one\n* ‸");
        assert_eq!(enter_at("3. one‸"), "3. one\n4. ‸");
        assert_eq!(enter_at("1) one‸"), "1) one\n2) ‸");
        assert_eq!(enter_at("- [x] done‸"), "- [x] done\n- [ ] ‸");
        assert_eq!(enter_at("- on‸e"), "- on\n- ‸e");
        assert_eq!(enter_at("- a\n  - b‸"), "- a\n  - b\n  - ‸");
        assert_eq!(enter_at("- a\n\n- b‸"), "- a\n\n- b\n\n- ‸");
    }

    #[test]
    fn enter_in_an_empty_item_ends_or_outdents() {
        assert_eq!(enter_at("- a\n- ‸"), "- a\n\n¶");
        assert_eq!(enter_at("- a\n- ‸\n- c"), "- a\n\n¶\n\n- c");
        // (An empty `  - ` right under `- a` would be a setext underline.)
        assert_eq!(enter_at("- a\n  - b\n  - ‸"), "- a\n  - b\n- ‸");
    }

    #[test]
    fn enter_in_quotes() {
        // A new paragraph in the quote, which gets its source once typed into.
        assert_eq!(enter_at("> one‸"), "> one\n>\n> ¶");
        assert_eq!(enter_at("> > deep‸"), "> > deep\n> >\n> > ¶");
        assert_eq!(enter_at("> one‸two"), "> one\n>\n> ‸two");
        // Lists in a quote.
        assert_eq!(enter_at("> - a‸"), "> - a\n> - ‸");
        assert_eq!(enter_at("> 1. a‸\n> 2. b"), "> 1. a\n> 2. ‸\n> 2. b");
        assert_eq!(enter_at("> - a\n> - ‸"), "> - a\n>\n> ¶");
        assert_eq!(enter_at("> - ‸"), "> ¶");
        // A heading in a quote.
        assert_eq!(enter_at("> # H‸\n>\n> p"), "> # H\n>\n> ¶\n>\n> p");
        // A quote in a list item.
        assert_eq!(enter_at("- a\n\n  > q‸"), "- a\n\n  > q\n  >\n  > ¶");
    }

    #[test]
    fn tab_and_backspace_in_a_quote() {
        assert_eq!(tab_at("> - a\n> - b‸", false), "> - a\n>   - b‸");
        assert_eq!(tab_at("> - a\n>   - b‸", true), "> - a\n> - b‸");
        assert_eq!(backspace_at("> - ‸a").as_deref(), Some("> ‸a"));
        // The first block of a quote leaves it; a later one joins the one before.
        assert_eq!(backspace_at("> ‸one\n>\n> two").as_deref(), Some("‸one\n\n> two"));
        assert_eq!(backspace_at("> one\n>\n> ‸two").as_deref(), Some("> one‸two"));
        assert_eq!(delete_at("> one‸\n>\n> two").as_deref(), Some("> one‸two"));
        assert_eq!(backspace_at("> > ‸deep").as_deref(), Some("> ‸deep"));
    }

    #[test]
    fn new_paragraphs_in_gaps_inside_a_quote() {
        let gap = |text: &str, at: usize| {
            let doc = Document::parse(text);
            let p = gap_pending(text, &doc, at, None);
            p.materialize(text, "x").0
        };
        // On a quote's blank line between two paragraphs.
        assert_eq!(gap("> a\n>\n> b", 5), "> a\n>\n> x\n>\n> b");
        // On a quote's blank first line.
        assert_eq!(gap(">\n> b", 1), "> x\n>\n> b");
        // Out of the quote: none.
        assert_eq!(gap("> a\n\nb", 4), "> a\n\nx\n\nb");
    }

    #[test]
    fn rules_in_the_blocks_of_a_list_item() {
        // A new paragraph in the item, under its text.
        assert_eq!(enter_at("- p1\n\n  p2‸\n- b"), "- p1\n\n  p2\n\n  ¶\n- b");
        assert_eq!(shift_enter_at("- p1\n\n  p2‸"), "- p1\n\n  p2\n  ‸");
        // A fence typed there is completed inside the item.
        assert_eq!(enter_at("- a\n\n  ```‸"), "- a\n\n  ```\n  ‸\n  ```");
        // Typed before other blocks, the fence is a paragraph until Enter (the editor masks it).
        let text = "- a\n\n  ```\n\n  > q";
        let fence = text.find('`').unwrap();
        let doc = Document::parse_masked(text, Some(fence));
        let region = region_at(&doc, fence + 3).unwrap();
        let e = nested(text, &doc, region, Sel::caret(fence + 3), enter).unwrap();
        assert_eq!(show(&e), "- a\n\n  ```\n  ‸\n  ```\n\n  > q");
        // The first block after the item's text joins it.
        assert_eq!(backspace_at("- p1\n\n  ‸p2").as_deref(), Some("- p1‸p2"));
        assert_eq!(delete_at("- p1‸\n\n  p2").as_deref(), Some("- p1‸p2"));
    }

    #[test]
    fn shift_enter_breaks_lines() {
        assert_eq!(shift_enter_at("one‸two"), "one\n‸two");
        assert_eq!(shift_enter_at("- one‸"), "- one\n  ‸");
        assert_eq!(shift_enter_at("1. one‸"), "1. one\n   ‸");
        assert_eq!(shift_enter_at("- [ ] one‸"), "- [ ] one\n  ‸");
        assert_eq!(shift_enter_at("> one‸"), "> one\n> ‸");
        assert_eq!(shift_enter_at("# H‸"), "# H\n\n¶");
    }

    #[test]
    fn backspace_removes_markers() {
        assert_eq!(backspace_at("## ‸Plan").as_deref(), Some("‸Plan"));
        assert_eq!(backspace_at("- ‸item").as_deref(), Some("‸item"));
        assert_eq!(backspace_at("- [ ] ‸item").as_deref(), Some("- ‸item"));
        assert_eq!(backspace_at("> ‸q").as_deref(), Some("‸q"));
        assert_eq!(backspace_at("1. ‸x").as_deref(), Some("‸x"));
        assert_eq!(backspace_at("ab‸c"), None);
    }

    #[test]
    fn backspace_joins_blocks() {
        assert_eq!(backspace_at("one\n\n‸two").as_deref(), Some("one‸two"));
        assert_eq!(backspace_at("- a\n‸- b").as_deref(), Some("- a‸b"));
        assert_eq!(backspace_at("a\n\n---\n\n‸b").as_deref(), Some("a\n\n‸b"));
        // Into a code block: only the cursor moves.
        assert_eq!(backspace_at("```\nx\n```\n\n‸b").as_deref(), Some("```\nx\n```‸\n\nb"));
        assert_eq!(backspace_at("‸first"), None);
    }

    #[test]
    fn delete_joins_the_next_block() {
        assert_eq!(delete_at("one‸\n\n# two").as_deref(), Some("one‸two"));
        assert_eq!(delete_at("o‸ne"), None);
        assert_eq!(delete_at("one‸\n\n***\n\ntwo").as_deref(), Some("one‸\n\ntwo"));
    }

    #[test]
    fn tab_nests_list_items_with_their_children() {
        assert_eq!(tab_at("- a\n- ‸b\n  - c\n- d", false), "- a\n  - ‸b\n    - c\n- d");
        assert_eq!(tab_at("1. a\n2. ‸b", false), "1. a\n   2. ‸b");
        // The first item can't be nested.
        assert_eq!(tab_at("- ‸a\n- b", false), "- ‸a\n- b");
        assert_eq!(tab_at("- a\n  - ‸b\n    - c", true), "- a\n- ‸b\n  - c");
        assert_eq!(tab_at("- ‸a", true), "- ‸a");
    }

    #[test]
    fn tab_in_code_indents() {
        assert_eq!(tab_at("```\n‸x\n```", false), "```\n    ‸x\n```");
        assert_eq!(tab_at("```\n    ‸x\n```", true), "```\n‸x\n```");
    }

    #[test]
    fn markers() {
        assert_eq!(line_marker("## a").map(|m| (m.kind, m.end)), Some((MarkerKind::Heading(2), 3)));
        assert_eq!(line_marker("#a"), None);
        assert_eq!(line_marker("  - [x] t").map(|m| (m.indent, m.end, m.task)), Some((2, 8, Some(4..8))));
        assert_eq!(line_marker("10. x").map(|m| m.kind), Some(MarkerKind::Ordered(10, '.')));
        assert_eq!(line_marker("-x"), None);
    }

    #[test]
    fn gaps_and_collapsing() {
        assert_eq!(Pending::gap("", 0).materialize("", "x").0, "x");
        assert_eq!(Pending::gap("a\n", 2).materialize("a\n", "x").0, "a\n\nx\n");
        assert_eq!(Pending::gap("a\n\n\nb", 3).materialize("a\n\n\nb", "x").0, "a\n\nx\n\nb");
        assert_eq!(collapse_blank_lines("a\n\n\n\nb", 3), ("a\n\nb".to_string(), 1));
        assert_eq!(collapse_blank_lines("a\n\n\n", 3), ("a\n".to_string(), 1));
        assert_eq!(collapse_blank_lines("\n\nb", 1), ("b".to_string(), 0));
    }

    #[test]
    fn pending_paragraphs_get_their_source_when_typed_into() {
        let p = Pending::after("one\n\ntwo", 3);
        assert_eq!(p.materialize("one\n\ntwo", "x"), ("one\n\nx\n\ntwo".to_string(), 5));
        let p = Pending::before(0);
        assert_eq!(p.materialize("one", "x").0, "x\n\none");
    }
}
