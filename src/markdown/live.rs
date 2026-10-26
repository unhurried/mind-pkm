//! The block being edited, as shown: rendered like the other blocks, except
//! the inline elements touching the cursor, whose Markdown is shown as well.
//!
//! The shown text is a sequence of units, each standing for a range of the
//! source: characters copied one to one (text, and the markers of revealed
//! elements) or one shown thing for a whole range (an emoji, an entity, a line
//! break written as `<br>`, an image or formula shown as an object). Source
//! bytes no unit shows are hidden; each belongs to the innermost inline
//! element around it, and is shown when that element is revealed. Hidden bytes
//! in no element are block syntax (`#`, list markers, `>`) and never shown.
//!
//! Positions convert both ways; edits on the shown text become source edits
//! that keep the markup of elements partly deleted.

use super::content::{self, Inline, Link, Style};
use super::parse::{BlockKind, ContainerKind, Document, prefix_len};
use pulldown_cmark::Event;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// The character an object (image, formula...) is shown as in the text.
pub const OBJECT: char = '\u{FFFC}';

/// Something shown as one object.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Image { src: String, alt: String, title: String, width: Option<f32>, link: Option<Link> },
    Math { tex: String, display: bool },
    Footnote { label: String, number: usize },
    Embed { tag: String, src: Option<String> },
}

/// How a unit is shown.
#[derive(Debug, Clone, PartialEq)]
pub enum Look {
    /// Rendered text.
    Text {
        style: Style,
        link: Option<Link>,
    },
    /// Markdown shown at the cursor.
    Marker,
    Break,
    Object(Object),
    /// The rendered image or formula after its source at the cursor; it takes
    /// no place in the text.
    Preview(Object),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    /// Its bytes in the shown text (empty for a preview).
    pub shown: Range<usize>,
    pub source: Range<usize>,
    /// Each shown byte is the source byte at the same distance; else the
    /// unit stands for its whole source.
    pub one_to_one: bool,
    pub look: Look,
    /// For a marker, the element it is the Markdown of.
    owner: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Display {
    pub text: String,
    pub units: Vec<Unit>,
    /// The block's content in the source, without its block syntax.
    pub content: Range<usize>,
    /// The source ranges of the inline elements.
    pub elements: Vec<Range<usize>>,
    /// The elements shown with their Markdown.
    pub revealed: Vec<usize>,
    /// Hidden source runs and the element each belongs to (None: block syntax).
    hidden: Vec<(Range<usize>, Option<usize>)>,
}

type Events = [(Event<'static>, Range<usize>)];

/// Whether a block is edited in its rendered form: paragraphs, headings, list
/// items, (empty) quotes and footnote definitions holding only text
/// (code-like blocks, and items or footnotes starting with other blocks, are
/// edited as source).
pub fn editable(doc: &Document, block: usize) -> bool {
    use BlockKind as K;
    use pulldown_cmark::Tag;
    let Some(b) = doc.blocks.get(block) else { return false };
    matches!(b.kind, K::Paragraph | K::Heading(_) | K::ListItem(_) | K::Quote | K::Footnote(_))
        && !doc.block_events(block).iter().any(|(e, _)| {
            matches!(
                e,
                Event::Html(_)
                    | Event::Rule
                    | Event::Start(
                        Tag::CodeBlock(_)
                            | Tag::HtmlBlock
                            | Tag::Table(_)
                            | Tag::MetadataBlock(_)
                            | Tag::List(_)
                            | Tag::BlockQuote(_)
                    )
            )
        })
}

/// What a block is, to tell whether an edit made it something else: its kind
/// (whether a list item is a task included) and its containers.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    kind: std::mem::Discriminant<BlockKind>,
    task: bool,
    outer: Vec<ContainerKind>,
}

impl Shape {
    pub fn new(kind: &BlockKind, outer: Vec<ContainerKind>) -> Shape {
        let task = matches!(kind, BlockKind::ListItem(i) if i.task.is_some());
        Shape { kind: std::mem::discriminant(kind), task, outer }
    }

    pub fn of(doc: &Document, block: usize) -> Shape {
        Shape::new(&doc.blocks[block].kind, doc.outer_kinds(block))
    }
}

/// After an edit of the block being edited - `doc` now, the cursor at
/// `cursor`, `typed` inserted, the block `before` it was - the byte the
/// parse should not see while the element the edit made is not determined
/// yet: `segment`, where the block's text (or the cursor's line in it)
/// starts, so that a marker typed there shows as typed. None when the block
/// is what it was, when the space after a marker determines the element
/// (`# `, `- `, `1. `, `- [ ] `, `> `), when the new element shows text on
/// the cursor's line (`>x` is a quote), and for a math paragraph (complete
/// when its closing `$$` is typed). Line syntax (a fence, `---`, a setext
/// underline, `[toc]`, HTML) stays undetermined until Enter.
pub fn undetermined(doc: &Document, cursor: usize, segment: usize, before: &Shape, typed: &str) -> Option<usize> {
    use super::edit::{self, MarkerKind};
    if segment >= cursor {
        return None;
    }
    let region = edit::region_at(doc, cursor)?;
    let block = doc.blocks.iter().position(|b| b.range.start == region.start)?;
    let shape = Shape::of(doc, block);
    let kind = &doc.blocks[block].kind;
    if shape == *before || *kind == BlockKind::Math {
        return None;
    }
    let line = edit::line_range(&doc.text, cursor);
    let line_text = &doc.text[line.clone()];
    let rest = &line_text[prefix_len(line_text, shape.outer.iter().copied())..];
    let marker = edit::line_marker(rest).filter(|m| m.end == rest.len());
    let made = |m: &MarkerKind| match m {
        MarkerKind::Heading(_) => matches!(kind, BlockKind::Heading(_)),
        MarkerKind::Bullet(_) | MarkerKind::Ordered(..) => matches!(kind, BlockKind::ListItem(_)),
        MarkerKind::Quote => *kind == BlockKind::Quote || shape.outer.last() == Some(&ContainerKind::Quote),
    };
    if typed.ends_with([' ', '\t']) && cursor == line.end && marker.is_some_and(|m| made(&m.kind)) {
        return None;
    }
    if editable(doc, block) {
        let d = Display::new(doc, doc.block_events(block), region, cursor);
        let shows = d.units.iter().any(|u| {
            !u.shown.is_empty()
                && matches!(u.look, Look::Text { .. } | Look::Object(_))
                && u.source.start < line.end
                && u.source.end > line.start
        });
        if shows {
            return None;
        }
    }
    let byte = *doc.text.as_bytes().get(segment)?;
    let masked = Document::parse_masked(&doc.text, Some(segment));
    let still = edit::region_at(&masked, cursor)
        .and_then(|r| masked.blocks.iter().position(|b| b.range.start == r.start))
        .is_some_and(|b| editable(&masked, b));
    (byte.is_ascii() && byte != b'\n' && still).then_some(segment)
}

impl Display {
    /// The block `region` of `doc` (its events `events`) shown with the
    /// elements touching the source offset `cursor` revealed.
    pub fn new(doc: &Document, events: &Events, region: Range<usize>, cursor: usize) -> Display {
        let (inlines, elements) = content::inlines_and_elements(doc, events);
        let text = &doc.text;
        // What the rendered runs show.
        let mut covered: Vec<Range<usize>> = Vec::new();
        for inline in &inlines {
            match inline {
                Inline::Text { map, .. } => covered.extend(map.pieces().map(|(_, s)| s)),
                other => covered.push(other.source()),
            }
        }
        covered.retain(|r| !r.is_empty());
        covered.sort_by_key(|r| r.start);
        let hidden = hidden_runs(&region, &covered, &elements);
        let revealed: Vec<usize> =
            (0..elements.len()).filter(|&e| elements[e].start <= cursor && cursor <= elements[e].end).collect();
        let is_revealed = |e: Option<usize>| e.is_some_and(|e| revealed.contains(&e));
        let element_of = |r: &Range<usize>| elements.iter().position(|e| e == r);

        // Items in source order: rendered runs and hidden runs.
        enum Item<'a> {
            Piece { text: &'a str, source: Range<usize>, style: Style, link: &'a Option<Link> },
            Inline(&'a Inline),
            Hidden(Range<usize>, Option<usize>),
        }
        let mut items: Vec<(usize, Item)> = Vec::new();
        for inline in &inlines {
            match inline {
                Inline::Text { text: t, style, link, map } => {
                    for (tr, sr) in map.pieces() {
                        items.push((sr.start, Item::Piece { text: &t[tr], source: sr, style: *style, link }));
                    }
                }
                other => items.push((other.source().start, Item::Inline(other))),
            }
        }
        for (r, owner) in &hidden {
            items.push((r.start, Item::Hidden(r.clone(), *owner)));
        }
        items.sort_by_key(|(start, item)| (*start, matches!(item, Item::Hidden(..)) as u8));

        let mut d = Display { elements: elements.clone(), revealed: revealed.clone(), ..Display::default() };
        let line_end = |r: &Range<usize>| {
            r.end >= region.end
                || text[r.end..].starts_with('\n')
                || text[r.end..].trim_start_matches([' ', '\t']).is_empty()
        };
        for (_, item) in items {
            match item {
                Item::Piece { text: t, source, style, link } => {
                    if source.is_empty() {
                        // Text the parser adds (spaces of an HTML block): nothing to edit.
                        continue;
                    }
                    if t.len() == source.len() {
                        d.push(t, source, true, Look::Text { style, link: link.clone() }, None);
                    } else if let Some(e) = element_of(&source).filter(|e| revealed.contains(e)) {
                        d.push(&text[source.clone()], source, true, Look::Marker, Some(e));
                    } else if !t.is_empty() {
                        d.push(t, source, false, Look::Text { style, link: link.clone() }, None);
                    }
                }
                Item::Inline(inline) => {
                    let source = inline.source();
                    let e =
                        elements.iter().position(|e| e.start <= source.start && source.end <= e.end && *e != (0..0));
                    let e = e.filter(|&e| elements[e] == source || matches!(inline, Inline::Embed { .. }));
                    let object = match inline {
                        Inline::Break { .. } => None,
                        Inline::Math { tex, display, .. } => Some(Object::Math { tex: tex.clone(), display: *display }),
                        Inline::Image { src, alt, title, width, link, .. } => Some(Object::Image {
                            src: src.clone(),
                            alt: alt.clone(),
                            title: title.clone(),
                            width: *width,
                            link: link.clone(),
                        }),
                        Inline::FootnoteRef { label, number, .. } => {
                            Some(Object::Footnote { label: label.clone(), number: *number })
                        }
                        Inline::Embed { tag, src, .. } => Some(Object::Embed { tag: tag.clone(), src: src.clone() }),
                        Inline::Text { .. } => None,
                    };
                    if is_revealed(e) {
                        d.push(&text[source.clone()], source.clone(), true, Look::Marker, e);
                        if let Some(o @ (Object::Image { .. } | Object::Math { .. })) = object {
                            d.preview(source, o);
                        }
                    } else {
                        match object {
                            Some(o) => d.push(&OBJECT.to_string(), source, false, Look::Object(o), None),
                            None => {
                                let one = &text[source.clone()] == "\n";
                                d.push("\n", source, one, Look::Break, None)
                            }
                        }
                    }
                }
                Item::Hidden(r, owner) => {
                    let t = &text[r.clone()];
                    if is_revealed(owner) {
                        d.push(t, r, true, Look::Marker, owner);
                    } else if owner.is_none()
                        && r.end == region.end
                        && t.contains('\n')
                        && t.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '>'))
                    {
                        // A line break typed at the end (Shift+Enter): the new line to type on.
                        let breaks = "\n".repeat(t.matches('\n').count());
                        d.push(&breaks, r, false, Look::Break, None);
                    } else if owner.is_none() && t.trim_matches([' ', '\t']).is_empty() && line_end(&r) {
                        // Spaces typed at the end of a line, which the parser drops.
                        d.push(&text[r.clone()], r, true, Look::Text { style: Style::default(), link: None }, None);
                    }
                }
            }
        }
        // The content: inside the block syntax at both ends.
        let mut start = region.start;
        let mut end = region.end;
        for (r, owner) in &hidden {
            if owner.is_none() && r.start == start && !d.units.iter().any(|u| u.source.start == r.start) {
                start = r.end;
            }
        }
        for (r, owner) in hidden.iter().rev() {
            if owner.is_none()
                && r.end == end
                && end > start
                && !d.units.iter().any(|u| u.source.end == r.end && !u.source.is_empty())
            {
                end = r.start.max(start);
            }
        }
        d.content = start..end;
        d.hidden = hidden;
        d
    }

    /// A table cell's Markdown (`<br>` and `\|` as written) shown with the
    /// elements touching `cursor` revealed; positions are in the cell's text.
    /// The cell is parsed as the only cell of a table, so that nothing in it
    /// is read as block syntax.
    pub fn for_cell(cell: &str, cursor: usize) -> Display {
        let wrapped = format!("|{cell}|\n|-|\n");
        let doc = Document::parse(&wrapped);
        let Some(block) = doc.blocks.iter().position(|b| b.kind == BlockKind::Table) else {
            return Display::plain(cell);
        };
        let d = Display::new(&doc, doc.block_events(block), 1..1 + cell.len(), cursor + 1);
        let back = |r: &Range<usize>| r.start.saturating_sub(1)..r.end.saturating_sub(1);
        Display {
            text: d.text,
            units: d.units.into_iter().map(|u| Unit { source: back(&u.source), ..u }).collect(),
            content: back(&d.content),
            elements: d.elements.iter().map(back).collect(),
            revealed: d.revealed,
            hidden: d.hidden.iter().map(|(r, o)| (back(r), *o)).collect(),
        }
    }

    /// Nothing shown, at the source offset `at` (a new empty paragraph).
    pub fn empty(at: usize) -> Display {
        Display { content: at..at, ..Display::default() }
    }

    /// Text shown as it is.
    fn plain(text: &str) -> Display {
        let mut d = Display { content: 0..text.len(), ..Display::default() };
        if !text.is_empty() {
            d.push(text, 0..text.len(), true, Look::Text { style: Style::default(), link: None }, None);
        }
        d
    }

    fn push(&mut self, shown: &str, source: Range<usize>, one_to_one: bool, look: Look, owner: Option<usize>) {
        let start = self.text.len();
        self.text.push_str(shown);
        // The second line break between two paragraphs (of a quote, a loose item) joins the first.
        if look == Look::Break
            && let Some(last) = self.units.last_mut()
            && last.look == Look::Break
            && last.source.end == source.start
            && source.is_empty()
        {
            last.shown.end = self.text.len();
            last.one_to_one = false;
            return;
        }
        // Consecutive markers and text pieces of the same look merge.
        if let Some(last) = self.units.last_mut()
            && one_to_one
            && last.one_to_one
            && last.look == look
            && last.owner == owner
            && last.source.end == source.start
            && last.shown.end == start
        {
            last.shown.end = self.text.len();
            last.source.end = source.end;
            return;
        }
        self.units.push(Unit { shown: start..self.text.len(), source, one_to_one, look, owner });
    }

    fn preview(&mut self, source: Range<usize>, object: Object) {
        let at = self.text.len();
        self.units.push(Unit {
            shown: at..at,
            source: source.end..source.end,
            one_to_one: false,
            look: Look::Preview(object),
            owner: None,
        });
    }

    fn text_units(&self) -> impl Iterator<Item = &Unit> {
        self.units.iter().filter(|u| !u.shown.is_empty())
    }

    /// The source offset of a shown offset: right after the source of the
    /// character before it (the content's start for offset 0).
    pub fn to_source(&self, d: usize) -> usize {
        if d == 0 {
            return self.text_units().next().map_or(self.content.start, |u| u.source.start);
        }
        for u in self.text_units() {
            if u.shown.start < d && d <= u.shown.end {
                return if u.one_to_one { u.source.start + (d - u.shown.start) } else { u.source.end };
            }
        }
        self.text_units().last().map_or(self.content.end, |u| u.source.end)
    }

    /// The shown offset of a source offset (a hidden one: where the next unit starts).
    pub fn to_shown(&self, s: usize) -> usize {
        for u in self.text_units() {
            if s <= u.source.start {
                return u.shown.start;
            }
            if s < u.source.end {
                return if u.one_to_one { u.shown.start + (s - u.source.start) } else { u.shown.end };
            }
        }
        self.text.len()
    }

    /// Whether the cursor can be at the shown offset `d` (not inside a unit
    /// standing for its source as a whole).
    pub fn is_stop(&self, d: usize) -> bool {
        self.text.is_char_boundary(d.min(self.text.len()))
            && !self.text_units().any(|u| !u.one_to_one && u.shown.start < d && d < u.shown.end)
    }

    /// The unit holding the shown byte `d`.
    fn unit_at(&self, d: usize) -> Option<&Unit> {
        self.text_units().find(|u| u.shown.start <= d && d < u.shown.end)
    }

    /// The source offset of the shown byte `d` (where its character starts).
    fn source_at(&self, d: usize) -> usize {
        match self.unit_at(d) {
            Some(u) if u.one_to_one => u.source.start + (d - u.shown.start),
            Some(u) => u.source.start,
            None => self.to_source(d),
        }
    }

    /// The source offset one character to the left of `s`, None at the start.
    pub fn left(&self, s: usize) -> Option<usize> {
        let d = self.to_shown(s);
        let prev = self.text[..d].grapheme_indices(true).next_back()?.0;
        Some(self.source_at(prev))
    }

    /// The source offset one character to the right of `s`, None at the end.
    pub fn right(&self, s: usize) -> Option<usize> {
        let d = self.to_shown(s);
        let (_, g) = self.text[d..].grapheme_indices(true).next()?;
        let u = self.unit_at(d)?;
        let next = (d + g.len()).min(u.shown.end);
        Some(if u.one_to_one { u.source.start + (next - u.shown.start) } else { u.source.end })
    }

    /// The source offset at the start of the word before `s` (Ctrl+Left).
    pub fn word_left(&self, s: usize) -> Option<usize> {
        let d = self.to_shown(s);
        if d == 0 {
            return None;
        }
        let trimmed = self.text[..d].trim_end();
        let Some(last) = trimmed.chars().next_back() else { return Some(self.to_source(0)) };
        let class = super::format::word_class(last);
        let mut start = trimmed.len() - last.len_utf8();
        if class.is_some() {
            while let Some(c) = trimmed[..start].chars().next_back() {
                if super::format::word_class(c) != class {
                    break;
                }
                start -= c.len_utf8();
            }
        }
        Some(self.source_at(start))
    }

    /// The source offset after the word after `s` (Ctrl+Right).
    pub fn word_right(&self, s: usize) -> Option<usize> {
        let d = self.to_shown(s);
        let rest = &self.text[d..];
        let skipped = rest.len() - rest.trim_start().len();
        let after = &rest[skipped..];
        let first = after.chars().next()?;
        let class = super::format::word_class(first);
        let mut len = first.len_utf8();
        if class.is_some() {
            for c in after[len..].chars() {
                if super::format::word_class(c) != class {
                    break;
                }
                len += c.len_utf8();
            }
        }
        Some(self.to_source(d + skipped + len))
    }

    /// The shown word around the shown offset `d` (double click).
    pub fn word_at(&self, d: usize) -> Option<Range<usize>> {
        super::format::word_at(&self.text, d)
    }

    /// The previous / next character's shown range, for Backspace / Delete.
    pub fn grapheme_before(&self, d: usize) -> Option<Range<usize>> {
        let (i, _) = self.text[..d].grapheme_indices(true).next_back()?;
        Some(i..d)
    }

    pub fn grapheme_after(&self, d: usize) -> Option<Range<usize>> {
        let (_, g) = self.text[d..].grapheme_indices(true).next()?;
        Some(d..d + g.len())
    }

    /// The source ranges deleting the shown range `shown` removes: the
    /// sources of its units; Markdown between them when its element lies
    /// inside or loses all of its content (then the whole element goes);
    /// block syntax between them.
    pub fn deletion(&self, shown: Range<usize>) -> Vec<Range<usize>> {
        let mut parts: Vec<Range<usize>> = Vec::new();
        for u in self.text_units() {
            let (a, b) = (shown.start.max(u.shown.start), shown.end.min(u.shown.end));
            if a >= b {
                continue;
            }
            if u.one_to_one {
                parts.push(u.source.start + (a - u.shown.start)..u.source.start + (b - u.shown.start));
            } else {
                parts.push(u.source.clone());
            }
        }
        let (Some(first), Some(last)) = (parts.iter().map(|r| r.start).min(), parts.iter().map(|r| r.end).max()) else {
            return Vec::new();
        };
        let within = |r: &Range<usize>| first <= r.start && r.end <= last;
        for (r, owner) in &self.hidden {
            if within(r) && owner.is_none_or(|e| within(&self.elements[e])) {
                parts.push(r.clone());
            }
        }
        // Elements left without content go as a whole.
        loop {
            let deleted = merge(parts.clone());
            let covered =
                |r: &Range<usize>| r.is_empty() || deleted.iter().any(|d| d.start <= r.start && r.end <= d.end);
            let touched = |r: &Range<usize>| deleted.iter().any(|d| d.start < r.end && r.start < d.end);
            let mut grew = false;
            for (e, range) in self.elements.iter().enumerate() {
                if covered(range) || !touched(range) {
                    continue;
                }
                let mut content = self
                    .text_units()
                    .filter(|u| range.start <= u.source.start && u.source.end <= range.end && u.owner != Some(e));
                if content.all(|u| covered(&u.source)) {
                    parts.push(range.clone());
                    grew = true;
                }
            }
            if !grew {
                return deleted;
            }
        }
    }

    /// The shown ranges of source ranges (for find matches).
    pub fn shown_ranges(&self, source: &[Range<usize>]) -> Vec<Range<usize>> {
        source.iter().map(|r| self.to_shown(r.start)..self.to_shown(r.end)).filter(|r| !r.is_empty()).collect()
    }
}

/// Sorted, merged ranges.
fn merge(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.retain(|r| !r.is_empty());
    ranges.sort_by_key(|r| r.start);
    let mut out: Vec<Range<usize>> = Vec::new();
    for r in ranges {
        match out.last_mut() {
            Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
            _ => out.push(r),
        }
    }
    out
}

/// The parts of `region` not `covered`, split where the innermost element
/// around them changes, with that element.
fn hidden_runs(
    region: &Range<usize>,
    covered: &[Range<usize>],
    elements: &[Range<usize>],
) -> Vec<(Range<usize>, Option<usize>)> {
    let mut gaps = Vec::new();
    let mut pos = region.start;
    for c in covered {
        if c.start > pos {
            gaps.push(pos..c.start.min(region.end));
        }
        pos = pos.max(c.end);
    }
    if pos < region.end {
        gaps.push(pos..region.end);
    }
    let innermost = |at: usize| {
        elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.start <= at && at < e.end)
            .min_by_key(|(_, e)| e.len())
            .map(|(i, _)| i)
    };
    let mut out: Vec<(Range<usize>, Option<usize>)> = Vec::new();
    for gap in gaps.into_iter().filter(|g| !g.is_empty()) {
        // Split at element boundaries inside the gap.
        let mut cuts: Vec<usize> =
            elements.iter().flat_map(|e| [e.start, e.end]).filter(|&b| gap.start < b && b < gap.end).collect();
        cuts.push(gap.end);
        cuts.sort_unstable();
        cuts.dedup();
        let mut start = gap.start;
        for cut in cuts {
            let owner = innermost(start);
            match out.last_mut() {
                Some((r, o)) if r.end == start && *o == owner => r.end = cut,
                _ => out.push((start..cut, owner)),
            }
            start = cut;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The display of the first block of `text` with the cursor at the `‸`.
    fn shown(source: &str) -> (Display, String, usize) {
        let cursor = source.find('‸').unwrap_or(source.len());
        let text = source.replace('‸', "");
        let doc = Document::parse(&text);
        let block = doc.block_at(cursor.min(text.len().saturating_sub(1))).unwrap_or(0);
        let region = super::super::edit::region_at(&doc, cursor).unwrap_or(doc.blocks[block].range.clone());
        let d = Display::new(&doc, doc.block_events(block), region, cursor);
        (d, text, cursor)
    }

    fn shown_text(source: &str) -> String {
        shown(source).0.text
    }

    #[test]
    fn only_the_elements_at_the_cursor_show_their_markdown() {
        assert_eq!(shown_text("some **bold** and [link](x.md) te‸xt"), "some bold and link text");
        assert_eq!(shown_text("some **bo‸ld** and [link](x.md) text"), "some **bold** and link text");
        assert_eq!(shown_text("some **bold** and [li‸nk](x.md) text"), "some bold and [link](x.md) text");
        // At an element's edge it is revealed too.
        assert_eq!(shown_text("some ‸**bold** text"), "some **bold** text");
        assert_eq!(shown_text("some **bold**‸ text"), "some **bold** text");
        // Nested elements around the cursor.
        assert_eq!(shown_text("**a _b‸_ c** d"), "**a _b_ c** d");
        assert_eq!(shown_text("**a _b_ c‸** d"), "**a b c** d");
    }

    #[test]
    fn block_markers_are_never_shown() {
        assert_eq!(shown_text("## Pla‸n"), "Plan");
        assert_eq!(shown_text("- [ ] ta‸sk"), "task");
        assert_eq!(shown_text("1. one‸"), "one");
        assert_eq!(shown_text("> quo‸ted\n> lines"), "quoted\nlines");
        let (d, _, _) = shown("## Pla‸n");
        assert_eq!(d.content, 3..7);
        assert_eq!(d.to_source(0), 3);
    }

    #[test]
    fn other_inline_elements() {
        assert_eq!(shown_text("a `code` b‸"), "a code b");
        assert_eq!(shown_text("a `co‸de` b"), "a `code` b");
        assert_eq!(shown_text("x ==mark== y‸"), "x mark y");
        assert_eq!(shown_text("x ==ma‸rk== y"), "x ==mark== y");
        assert_eq!(shown_text("H~2~O x^2^ z‸"), "H2O x2 z");
        assert_eq!(shown_text("H~2‸~O"), "H~2~O");
        assert_eq!(shown_text("<u>under</u> l‸"), "under l");
        assert_eq!(shown_text("<u>und‸er</u> l"), "<u>under</u> l");
        assert_eq!(shown_text("a \\* b‸"), "a * b");
        assert_eq!(shown_text("a \\*‸ b"), "a \\* b");
        assert_eq!(shown_text("a &amp; b‸"), "a & b");
        assert_eq!(shown_text(":smile: x‸"), "😄 x");
        assert_eq!(shown_text("~~gone~~ x‸"), "gone x");
        assert_eq!(shown_text("<https://x.org> y‸"), "https://x.org y");
    }

    #[test]
    fn images_and_math_are_objects_until_the_cursor_reaches_them() {
        let (d, _, _) = shown("see ![map](m.png) and $x^2$ end‸");
        assert_eq!(d.text, format!("see {OBJECT} and {OBJECT} end"));
        assert!(matches!(&d.units[1].look, Look::Object(Object::Image { src, .. }) if src == "m.png"));
        assert!(matches!(&d.units[3].look, Look::Object(Object::Math { tex, .. }) if tex == "x^2"));
        let (d, _, _) = shown("see ![map](m.png)‸ and $x^2$ end");
        assert_eq!(d.text, format!("see ![map](m.png) and {OBJECT} end"));
        assert!(d.units.iter().any(|u| matches!(&u.look, Look::Preview(Object::Image { .. }))));
    }

    #[test]
    fn which_blocks_are_edited_rendered() {
        let editable = |text: &str| {
            let doc = Document::parse(text);
            (0..doc.blocks.len()).map(|b| editable(&doc, b)).collect::<Vec<_>>()
        };
        assert_eq!(editable("# h\n\npara\n\n- a\n  - b\n\n> q\n\n[^1]: note"), vec![true; 6]);
        assert_eq!(editable("```\nx\n```\n\n$$\nx\n$$\n\n<div>\nx\n</div>\n\n---"), vec![false; 4]);
        assert_eq!(
            editable("> - in a quote\n>\n> > nested\n\n- a\n\n  ```\n  x\n  ```"),
            vec![true, true, true, false]
        );
        assert_eq!(editable("| a |\n| - |\n| 1 |"), vec![false]);
        assert_eq!(editable("see\n$$x^2$$\nhere"), vec![true]);
        assert_eq!(editable("- > first block of an item"), vec![false]);
    }

    #[test]
    fn table_cells() {
        let d = Display::for_cell("**done** to<br>day", 18);
        assert_eq!(d.text, "done to\nday");
        assert_eq!(d.to_source(d.text.len()), 18);
        let d = Display::for_cell("**do‸ne** x".replace('‸', "").as_str(), 4);
        assert_eq!(d.text, "**done** x");
        // Block syntax is text in a cell; escaped pipes are pipes.
        assert_eq!(Display::for_cell("1. # x", 0).text, "1. # x");
        assert_eq!(Display::for_cell("a\\|b", 0).text, "a|b");
        assert_eq!(Display::for_cell("", 0).text, "");
    }

    #[test]
    fn blocks_in_containers_without_their_syntax() {
        let (d, text, _) = shown("> - a\n> - b‸\n>   more");
        assert_eq!(d.text, "b\nmore");
        assert_eq!(&text[d.content.start..], "b\n>   more");
        // An empty quote: nothing shown, the cursor after its `>`.
        let (d, text, _) = shown("> ‸");
        assert_eq!((d.text.as_str(), d.to_source(0)), ("", text.len()));
        // A setext underline stays hidden.
        assert_eq!(shown_text("Title‸\n==="), "Title");
    }

    #[test]
    fn spaces_typed_at_the_end_stay_visible() {
        assert_eq!(shown_text("foo ‸"), "foo ");
    }

    #[test]
    fn positions_map_both_ways() {
        let (d, text, _) = shown("a **bold** c‸");
        assert_eq!(d.text, "a bold c");
        // Left-attached: after "a " is the bold's start; after "bold" inside it.
        assert_eq!(d.to_source(2), 2);
        assert_eq!(d.to_source(6), 8);
        assert_eq!(d.to_source(7), 11);
        assert_eq!(d.to_shown(5), 3);
        assert_eq!(d.to_shown(9), 6, "hidden: where the next unit starts");
        assert_eq!(&text[d.to_source(8)..], "");
    }

    #[test]
    fn moving_through_an_element() {
        // From "a" to the right: into the element before its markers.
        let (d, _, _) = shown("a‸ **b** c");
        assert_eq!(d.right(1), Some(2));
        let (d, _, _) = shown("a ‸**b** c");
        assert_eq!(d.text, "a **b** c");
        assert_eq!(d.right(2), Some(3));
        // From after the space to the left: right after the closing markers.
        let (d, _, _) = shown("a **b** ‸c");
        assert_eq!(d.text, "a b c");
        assert_eq!(d.left(8), Some(7));
    }

    fn delete(source: &str, what: &str) -> String {
        let (d, text, _) = shown(source);
        let at = d.text.find(what).unwrap();
        let mut out = text.clone();
        for r in d.deletion(at..at + what.len()).iter().rev() {
            out.replace_range(r.clone(), "");
        }
        out
    }

    #[test]
    fn deleting_keeps_the_markup_of_what_remains() {
        // The cursor elsewhere: the element is rendered.
        assert_eq!(delete("x **bold** y‸", "bo"), "x **ld** y");
        assert_eq!(delete("x **bold** y‸", "x bo"), "**ld** y");
        assert_eq!(delete("x **bold** y‸", "ld y"), "x **bo**");
        // All of its content: the element goes.
        assert_eq!(delete("a **bold** b‸", "bold"), "a  b");
        assert_eq!(delete("a **bold** b‸", "a bold"), " b");
        // Markers shown at the cursor are deleted like text.
        assert_eq!(delete("a **bo‸ld** b", "**b"), "a old** b");
        // An object goes with its source.
        assert_eq!(delete("see ![m](m.png) end‸", &format!("{OBJECT} ")), "see end");
        // Across lines of a quote: the line's marker goes too.
        assert_eq!(delete("> one‸\n> two", "e\nt"), "> onwo");
    }
}
