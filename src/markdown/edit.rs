//! The editing rules of the page editor that go beyond inserting characters:
//! what Enter, Shift+Enter, Backspace, Delete and Tab do depending on the
//! block, as pure functions from the page text and the cursor to the new
//! text and cursor.
//!
//! The cursor is in the *active region*: the source the editor shows, which is
//! a block (or a few, while text is pasted or typed). A new empty paragraph
//! has no source until something is typed into it; it is `Pending`.

use super::parse::{BlockKind, Document};
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
}

impl Pending {
    /// An empty paragraph after the block ending at `at`, separated from
    /// what follows by a blank line.
    fn after(text: &str, at: usize) -> Pending {
        let rest = &text[at..];
        let newlines = rest.len() - rest.trim_start_matches('\n').len();
        let suffix = if rest.trim().is_empty() { 0 } else { 2usize.saturating_sub(newlines) };
        Pending { at, prefix: "\n\n".into(), suffix: "\n".repeat(suffix) }
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
        Pending { at, prefix: "\n".repeat(prefix), suffix: "\n".repeat(suffix) }
    }

    fn before(at: usize) -> Pending {
        Pending { at, prefix: String::new(), suffix: "\n\n".into() }
    }

    /// The text with `typed` put in the paragraph, and where `typed` starts.
    pub fn materialize(&self, text: &str, typed: &str) -> (String, usize) {
        let mut out = String::with_capacity(text.len() + typed.len() + 4);
        out.push_str(&text[..self.at]);
        out.push_str(&self.prefix);
        let start = out.len();
        out.push_str(typed);
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

/// The region of the block holding `offset`: its range, extended to `offset`
/// when only spaces follow the block on its line (`- ` of a new item).
pub fn region_at(doc: &Document, offset: usize) -> Option<Range<usize>> {
    if let Some(b) = doc.block_at(offset) {
        return Some(doc.blocks[b].range.clone());
    }
    let b = doc.blocks.iter().rev().find(|b| b.range.end <= offset)?;
    let between = &doc.text[b.range.end..offset];
    (between.chars().all(|c| c == ' ' || c == '\t')).then_some(b.range.start..offset)
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
fn content_start(text: &str, block_start: usize) -> usize {
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
        // Under the previous item of the same list.
        let prev = doc.blocks[..index].iter().rev().find_map(|b| match &b.kind {
            BlockKind::ListItem(p) if !p.continuation && p.depth <= item.depth => Some((b, p.depth)),
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
        show(&enter(&text, &doc, region, Sel::caret(c)).expect("handled"))
    }

    fn shift_enter_at(marked: &str) -> String {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        show(&shift_enter(&text, &doc, region, Sel::caret(c)).expect("handled"))
    }

    fn backspace_at(marked: &str) -> Option<String> {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        backspace(&text, &doc, region, c).map(|e| show(&e))
    }

    fn delete_at(marked: &str) -> Option<String> {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        delete(&text, &doc, region, c).map(|e| show(&e))
    }

    fn tab_at(marked: &str, outdent: bool) -> String {
        let (text, region, c) = setup(marked);
        let doc = Document::parse(&text);
        show(&indent(&text, &doc, region, Sel::caret(c), outdent).expect("handled"))
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
        assert_eq!(enter_at("> one‸"), "> one\n>\n> ‸");
        assert_eq!(enter_at("> > deep‸"), "> > deep\n> >\n> > ‸");
        assert_eq!(enter_at("> one\n>\n> ‸"), "> one\n\n¶");
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
