//! The content of a block as the editor renders it: nested nodes (for quotes,
//! lists, tables) down to styled inline runs. Every run remembers the source
//! bytes it came from, so that a click on rendered text can be mapped back to
//! the source.
//!
//! On top of what the parser reports, this recognizes Typora's `==highlight==`,
//! `:emoji:` shortcodes, bare URLs, and a subset of inline HTML.

use super::parse::Document;
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Tag, TagEnd};
use std::ops::Range;

/// RGB color, from inline HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub underline: bool,
    pub code: bool,
    pub highlight: bool,
    pub sub: bool,
    pub sup: bool,
    pub kbd: bool,
    pub color: Option<Rgb>,
    /// HTML the editor doesn't render, shown as written.
    pub dim: bool,
}

impl Style {
    pub fn is_plain(&self) -> bool {
        *self == Style::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub dest: String,
    pub title: String,
}

/// How a piece of rendered text maps to the source: pieces of `(text start,
/// source range)`, in order. Within a piece whose text is as long as its
/// source, offsets map one to one; otherwise to the piece's start or end.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceMap {
    pieces: Vec<(usize, Range<usize>)>,
    len: usize,
}

/// A map of a text of `text_len` bytes taken one to one from `source`
/// (where the lengths match; else to its ends).
pub fn source_map(text_len: usize, source: Range<usize>) -> SourceMap {
    SourceMap::new(text_len, source)
}

impl SourceMap {
    fn new(text_len: usize, source: Range<usize>) -> Self {
        SourceMap { pieces: vec![(0, source)], len: text_len }
    }

    fn append(&mut self, other: &SourceMap) {
        for (start, source) in &other.pieces {
            self.pieces.push((self.len + start, source.clone()));
        }
        self.len += other.len;
    }

    /// The map of the text `range` of this one.
    pub fn slice(&self, range: Range<usize>) -> SourceMap {
        let start = self.source_of(range.start);
        let end = self.source_of(range.end);
        let mut pieces: Vec<(usize, Range<usize>)> = Vec::new();
        for (i, (text_start, source)) in self.pieces.iter().enumerate() {
            let text_end = self.pieces.get(i + 1).map_or(self.len, |p| p.0);
            if text_end <= range.start || *text_start >= range.end {
                continue;
            }
            let from = (*text_start).max(range.start);
            let s = if *text_start >= range.start { source.start } else { start };
            let e = if text_end <= range.end { source.end } else { end };
            pieces.push((from - range.start, s..e.max(s)));
        }
        if pieces.is_empty() {
            pieces.push((0, start..end.max(start)));
        }
        SourceMap { pieces, len: range.end - range.start }
    }

    /// The source offset of a text offset.
    pub fn source_of(&self, offset: usize) -> usize {
        let i = self.pieces.partition_point(|p| p.0 <= offset).saturating_sub(1);
        let (text_start, source) = &self.pieces[i];
        let text_end = self.pieces.get(i + 1).map_or(self.len, |p| p.0);
        let within = offset.saturating_sub(*text_start);
        if text_end - text_start == source.len() {
            source.start + within.min(source.len())
        } else if offset >= text_end {
            source.end
        } else if within == 0 {
            source.start
        } else {
            source.end
        }
    }

    /// The pieces as (text range, source range), in order.
    pub fn pieces(&self) -> impl Iterator<Item = (Range<usize>, Range<usize>)> + '_ {
        self.pieces.iter().enumerate().map(|(i, (start, source))| {
            let end = self.pieces.get(i + 1).map_or(self.len, |p| p.0);
            (*start..end, source.clone())
        })
    }

    /// The source bytes the whole text came from.
    pub fn source_range(&self) -> Range<usize> {
        let start = self.pieces.first().map_or(0, |p| p.1.start);
        let end = self.pieces.last().map_or(start, |p| p.1.end);
        start..end.max(start)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text {
        text: String,
        style: Style,
        link: Option<Link>,
        map: SourceMap,
    },
    /// A line break (hard or soft).
    Break {
        source: Range<usize>,
    },
    Math {
        tex: String,
        display: bool,
        link: Option<Link>,
        source: Range<usize>,
    },
    Image {
        src: String,
        alt: String,
        title: String,
        width: Option<f32>,
        link: Option<Link>,
        source: Range<usize>,
    },
    FootnoteRef {
        label: String,
        number: usize,
        source: Range<usize>,
    },
    /// `<iframe>`, `<video>`... shown as a placeholder.
    Embed {
        tag: String,
        src: Option<String>,
        source: Range<usize>,
    },
}

impl Inline {
    pub fn source(&self) -> Range<usize> {
        match self {
            Inline::Text { map, .. } => map.source_range(),
            Inline::Break { source }
            | Inline::Math { source, .. }
            | Inline::Image { source, .. }
            | Inline::FootnoteRef { source, .. }
            | Inline::Embed { source, .. } => source.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub task: Option<bool>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableNode {
    pub align: Vec<Alignment>,
    pub header: Vec<Vec<Inline>>,
    pub rows: Vec<Vec<Vec<Inline>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Paragraph(Vec<Inline>),
    Heading(u8, Vec<Inline>),
    Code { lang: Option<String>, text: String },
    Math(String),
    Quote(Vec<Node>),
    List { start: Option<u64>, items: Vec<Item> },
    Rule,
    Html(Vec<Inline>),
    Table(TableNode),
    FrontMatter(String),
    Footnote { number: usize, children: Vec<Node> },
}

type Events = [(Event<'static>, Range<usize>)];

/// The nodes of a block's events (or any slice of complete elements, with
/// inline events at its top making up a paragraph).
pub fn nodes(doc: &Document, events: &Events) -> Vec<Node> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut inline_start = None;
    let flush = |from: Option<usize>, to: usize, out: &mut Vec<Node>| {
        if let Some(from) = from {
            let inlines = inlines(doc, &events[from..to]);
            if !inlines.is_empty() {
                out.push(Node::Paragraph(inlines));
            }
        }
    };
    while i < events.len() {
        let (event, range) = &events[i];
        let block_start = match event {
            Event::Start(tag) => is_block_tag(tag),
            Event::Rule | Event::Html(_) => true,
            Event::TaskListMarker(_) => {
                i += 1;
                continue;
            }
            _ => false,
        };
        if !block_start {
            inline_start.get_or_insert(i);
            i += 1;
            continue;
        }
        flush(inline_start.take(), i, &mut out);
        match event {
            Event::Rule => {
                out.push(Node::Rule);
                i += 1;
            }
            Event::Html(_) => {
                // Html events outside an HTML block element: render as HTML.
                let start = i;
                while i < events.len() && matches!(events[i].0, Event::Html(_)) {
                    i += 1;
                }
                out.push(Node::Html(html_inlines(doc, &events[start..i])));
            }
            Event::Start(tag) => {
                let end = matching_end(events, i);
                let inner = &events[i + 1..end];
                out.push(match tag {
                    Tag::Paragraph => {
                        let content = nodes_inline_only(doc, inner);
                        if let [Inline::Math { tex, display: true, .. }] = content.as_slice() {
                            Node::Math(tex.clone())
                        } else {
                            Node::Paragraph(content)
                        }
                    }
                    Tag::Heading { level, .. } => {
                        Node::Heading(super::parse::heading_level(*level), nodes_inline_only(doc, inner))
                    }
                    Tag::BlockQuote(_) => Node::Quote(nodes(doc, inner)),
                    Tag::CodeBlock(kind) => Node::Code {
                        lang: match kind {
                            CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(str::to_string),
                            CodeBlockKind::Indented => None,
                        },
                        text: code_text(inner),
                    },
                    Tag::HtmlBlock => Node::Html(html_inlines(doc, inner)),
                    Tag::List(start) => Node::List { start: *start, items: items(doc, inner) },
                    Tag::Table(align) => Node::Table(table(doc, align, inner)),
                    Tag::MetadataBlock(_) => Node::FrontMatter(code_text(inner)),
                    Tag::FootnoteDefinition(label) => Node::Footnote {
                        number: doc.footnotes.get(label.as_ref()).copied().unwrap_or(0),
                        children: nodes(doc, inner),
                    },
                    _ => Node::Paragraph(inlines(doc, inner)),
                });
                let _ = range;
                i = end + 1;
            }
            _ => i += 1,
        }
    }
    flush(inline_start, events.len(), &mut out);
    out
}

/// The inline content of a paragraph or heading.
fn nodes_inline_only(doc: &Document, events: &Events) -> Vec<Inline> {
    inlines(doc, events)
}

fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::FootnoteDefinition(_)
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::MetadataBlock(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
    )
}

fn matching_end(events: &Events, start: usize) -> usize {
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
    events.len().saturating_sub(1)
}

fn code_text(events: &Events) -> String {
    events
        .iter()
        .filter_map(|(e, _)| match e {
            Event::Text(t) | Event::Html(t) => Some(t.as_ref()),
            _ => None,
        })
        .collect()
}

fn items(doc: &Document, events: &Events) -> Vec<Item> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < events.len() {
        if matches!(events[i].0, Event::Start(Tag::Item)) {
            let end = matching_end(events, i);
            let inner = &events[i + 1..end];
            let task = inner.iter().take(3).find_map(|(e, _)| match e {
                Event::TaskListMarker(c) => Some(*c),
                _ => None,
            });
            out.push(Item { task, children: nodes(doc, inner) });
            i = end + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// The cells of a table (`events` are the table's, without its start and end).
fn table(doc: &Document, align: &[Alignment], events: &Events) -> TableNode {
    let mut node = TableNode { align: align.to_vec(), header: Vec::new(), rows: Vec::new() };
    let mut i = 0;
    let mut in_head = false;
    let mut row: Vec<Vec<Inline>> = Vec::new();
    while i < events.len() {
        match &events[i].0 {
            Event::Start(Tag::TableHead) => {
                in_head = true;
                row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                in_head = false;
                node.header = std::mem::take(&mut row);
            }
            Event::Start(Tag::TableRow) => row.clear(),
            Event::End(TagEnd::TableRow) => node.rows.push(std::mem::take(&mut row)),
            Event::Start(Tag::TableCell) => {
                let end = matching_end(events, i);
                row.push(inlines(doc, &events[i + 1..end]));
                i = end;
            }
            _ => {}
        }
        i += 1;
    }
    let _ = in_head;
    node
}

/// The node of a whole table block.
pub fn table_block(doc: &Document, block: usize) -> TableNode {
    let b = &doc.blocks[block];
    let align = match doc.events.get(b.events.start.wrapping_sub(1)) {
        Some((Event::Start(Tag::Table(align)), _)) => align.clone(),
        _ => Vec::new(),
    };
    table(doc, &align, &doc.events[b.events.clone()])
}

/// One step of inline styling: what an inline tag or HTML element adds.
#[derive(Debug, Clone)]
enum Frame {
    Emphasis,
    Strong,
    Strike,
    Sub,
    Sup,
    Link(Link),
    Html { name: String, style: Style, link: Option<Link> },
}

/// Inline events turned into runs.
pub fn inlines(doc: &Document, events: &Events) -> Vec<Inline> {
    inlines_and_elements(doc, events).0
}

/// Inline events turned into runs, with the source ranges of the inline
/// elements they came from: what the parser tags (emphasis, links, images,
/// code spans, math, footnote references...), pairs of inline HTML tags,
/// `==`, `~` and `^` pairs, and every piece of text shown differently from
/// its source (an escape, an entity, an emoji shortcode).
pub fn inlines_and_elements(doc: &Document, events: &Events) -> (Vec<Inline>, Vec<Range<usize>>) {
    let mut b = InlineBuilder { out: Vec::new(), stack: Vec::new(), elements: Vec::new(), open_html: Vec::new() };
    let mut i = 0;
    while i < events.len() {
        let (event, range) = &events[i];
        match event {
            Event::Text(t) => {
                // Adjacent text events (the parser splits at `[`, `]`...) as one run.
                let mut text = t.to_string();
                let mut map = SourceMap::new(t.len(), range.clone());
                while let Some((Event::Text(next), r)) = events.get(i + 1) {
                    map.append(&SourceMap::new(next.len(), r.clone()));
                    text.push_str(next);
                    i += 1;
                }
                b.text(text, map);
            }
            Event::Code(t) => {
                b.elements.push(range.clone());
                let mut style = b.style();
                style.code = true;
                b.push_text(t.to_string(), style, SourceMap::new(t.len(), code_inner(&doc.text, range)));
            }
            Event::InlineMath(t) | Event::DisplayMath(t) => {
                b.elements.push(range.clone());
                b.out.push(Inline::Math {
                    tex: t.trim().to_string(),
                    display: matches!(event, Event::DisplayMath(_)),
                    link: b.link(),
                    source: range.clone(),
                })
            }
            Event::SoftBreak | Event::HardBreak => b.out.push(Inline::Break { source: range.clone() }),
            Event::FootnoteReference(label) => {
                b.elements.push(range.clone());
                b.out.push(Inline::FootnoteRef {
                    label: label.to_string(),
                    number: doc.footnotes.get(label.as_ref()).copied().unwrap_or(0),
                    source: range.clone(),
                })
            }
            Event::InlineHtml(html) | Event::Html(html) => b.html(html, range.clone()),
            Event::Start(
                tag @ (Tag::Emphasis
                | Tag::Strong
                | Tag::Strikethrough
                | Tag::Subscript
                | Tag::Superscript
                | Tag::Link { .. }),
            ) => {
                b.elements.push(range.clone());
                b.stack.push(match tag {
                    Tag::Emphasis => Frame::Emphasis,
                    Tag::Strong => Frame::Strong,
                    Tag::Strikethrough => Frame::Strike,
                    Tag::Subscript => Frame::Sub,
                    Tag::Superscript => Frame::Sup,
                    Tag::Link { dest_url, title, .. } => {
                        Frame::Link(Link { dest: dest_url.to_string(), title: title.to_string() })
                    }
                    _ => unreachable!(),
                })
            }
            Event::End(
                TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough
                | TagEnd::Subscript
                | TagEnd::Superscript
                | TagEnd::Link,
            ) => {
                // The innermost non-HTML frame.
                if let Some(p) = b.stack.iter().rposition(|f| !matches!(f, Frame::Html { .. })) {
                    b.stack.remove(p);
                }
            }
            Event::Start(Tag::Image { dest_url, title, .. }) => {
                b.elements.push(range.clone());
                let end = matching_end(events, i);
                let alt = super::parse::plain_text(&events[i + 1..end]);
                b.out.push(Inline::Image {
                    src: dest_url.to_string(),
                    alt,
                    title: title.to_string(),
                    width: None,
                    link: b.link(),
                    source: range.clone(),
                });
                i = end;
            }
            // Paragraphs of a loose list item, one after the other.
            Event::End(TagEnd::Paragraph) if events[i + 1..].iter().any(|(e, _)| !matches!(e, Event::End(_))) => {
                // The source between the paragraphs stands for the break.
                let next =
                    events[i + 1..].iter().find(|(e, _)| !matches!(e, Event::End(_))).map_or(range.end, |e| e.1.start);
                b.out.push(Inline::Break { source: range.end..next.max(range.end) });
                b.out.push(Inline::Break { source: next.max(range.end)..next.max(range.end) });
            }
            _ => {}
        }
        i += 1;
    }
    let mut elements = b.elements;
    // Escapes: the parser drops the backslash and starts a text at the escaped character.
    for (event, range) in events {
        if let Event::Text(t) = event
            && range.start > 0
            && doc.text.as_bytes()[range.start - 1] == b'\\'
            && t.chars().next().is_some_and(|c| c.is_ascii_punctuation() && doc.text[range.start..].starts_with(c))
        {
            elements.push(range.start - 1..range.start + 1);
        }
    }
    let out = sub_sup(highlight(b.out, &mut elements), &mut elements);
    // Pieces shown differently from their source (escapes, entities, emoji).
    for inline in &out {
        if let Inline::Text { map, .. } = inline {
            for (text, source) in map.pieces() {
                if text.len() != source.len() && !source.is_empty() {
                    elements.push(source);
                }
            }
        }
    }
    elements.sort_by_key(|r| (r.start, std::cmp::Reverse(r.end)));
    elements.dedup();
    (out, elements)
}

/// The inside of a code span's source, without the backticks.
fn code_inner(text: &str, range: &Range<usize>) -> Range<usize> {
    let source = &text[range.clone()];
    let ticks = source.len() - source.trim_start_matches('`').len();
    let inner = range.start + ticks..range.end - ticks.min(range.len() - ticks);
    // A space after the opening backticks is dropped by the parser when both ends have one.
    let s = &text[inner.clone()];
    if s.len() >= 2 && s.starts_with(' ') && s.ends_with(' ') && !s.trim().is_empty() {
        inner.start + 1..inner.end - 1
    } else {
        inner
    }
}

struct InlineBuilder {
    out: Vec<Inline>,
    stack: Vec<Frame>,
    elements: Vec<Range<usize>>,
    /// Open HTML elements: their name and where their opening tag starts.
    open_html: Vec<(String, usize)>,
}

impl InlineBuilder {
    fn style(&self) -> Style {
        let mut s = Style::default();
        for f in &self.stack {
            match f {
                Frame::Emphasis => s.italic = true,
                Frame::Strong => s.bold = true,
                Frame::Strike => s.strike = true,
                Frame::Sub => s.sub = true,
                Frame::Sup => s.sup = true,
                Frame::Link(_) => {}
                Frame::Html { style, .. } => {
                    s.bold |= style.bold;
                    s.italic |= style.italic;
                    s.strike |= style.strike;
                    s.underline |= style.underline;
                    s.highlight |= style.highlight;
                    s.sub |= style.sub;
                    s.sup |= style.sup;
                    s.kbd |= style.kbd;
                    s.code |= style.code;
                    if style.color.is_some() {
                        s.color = style.color;
                    }
                }
            }
        }
        s
    }

    fn link(&self) -> Option<Link> {
        self.stack.iter().rev().find_map(|f| match f {
            Frame::Link(l) => Some(l.clone()),
            Frame::Html { link: Some(l), .. } => Some(l.clone()),
            _ => None,
        })
    }

    fn push_text(&mut self, text: String, style: Style, map: SourceMap) {
        if !text.is_empty() {
            let link = self.link();
            self.out.push(Inline::Text { text, style, link, map });
        }
    }

    /// Text with emoji shortcodes and bare URLs recognized.
    fn text(&mut self, text: String, map: SourceMap) {
        let style = self.style();
        let link = self.link();
        self.out.extend(split_specials(&text, &map, style, link));
    }

    /// An HTML tag (or several, for an HTML block): supported ones change the
    /// style or insert an element; others are shown dimmed as written.
    fn html(&mut self, html: &str, range: Range<usize>) {
        let mut pos = 0;
        for token in html_tokens(html) {
            let r = range.start + token.range.start.min(range.len())..range.start + token.range.end.min(range.len());
            if token.range.start > pos {
                let t = html[pos..token.range.start].to_string();
                let style = self.style();
                let s = range.start + pos.min(range.len());
                self.push_text(t.clone(), style, SourceMap::new(t.len(), s..r.start));
            }
            pos = token.range.end;
            self.tag(&html[token.range.clone()], token, r);
        }
        if pos < html.len() {
            let t = html[pos..].to_string();
            let style = self.style();
            let s = range.start + pos.min(range.len());
            self.push_text(t.clone(), style, SourceMap::new(t.len(), s..range.end));
        }
    }

    fn tag(&mut self, raw: &str, token: HtmlToken, source: Range<usize>) {
        let name = token.name.as_str();
        let mut style = Style::default();
        let mut link = None;
        match (name, token.kind) {
            (_, TagKind::Comment) => {
                self.dim(raw, source);
                return;
            }
            ("br", _) => {
                self.elements.push(source.clone());
                self.out.push(Inline::Break { source });
                return;
            }
            ("img", TagKind::Open | TagKind::SelfClosing) => {
                self.elements.push(source.clone());
                self.out.push(Inline::Image {
                    src: token.attr("src").unwrap_or_default(),
                    alt: token.attr("alt").unwrap_or_default(),
                    title: token.attr("title").unwrap_or_default(),
                    width: token.attr("width").and_then(|w| w.trim_end_matches("px").parse().ok()),
                    link: self.link(),
                    source,
                });
                return;
            }
            ("iframe" | "video" | "audio" | "embed" | "object", TagKind::Open | TagKind::SelfClosing) => {
                let src = token.attr("src").or_else(|| token.attr("data"));
                if token.kind == TagKind::Open {
                    self.open_html.push((name.to_string(), source.start));
                } else {
                    self.elements.push(source.clone());
                }
                self.out.push(Inline::Embed { tag: name.to_string(), src, source });
                return;
            }
            ("iframe" | "video" | "audio" | "embed" | "object" | "source", _) => {
                // `<source>` and closing tags of embeds belong to the placeholder.
                if name == "source"
                    && let Some(Inline::Embed { src: src @ None, .. }) = self.out.last_mut()
                {
                    *src = token.attr("src");
                }
                if token.kind == TagKind::Close {
                    self.close_html(name, source.end);
                }
                return;
            }
            ("u" | "ins", _) => style.underline = true,
            ("s" | "del" | "strike", _) => style.strike = true,
            ("b" | "strong", _) => style.bold = true,
            ("i" | "em", _) => style.italic = true,
            ("mark", _) => style.highlight = true,
            ("sub", _) => style.sub = true,
            ("sup", _) => style.sup = true,
            ("kbd", _) => style.kbd = true,
            ("code", _) => style.code = true,
            ("span" | "font", _) => {
                style.color = token
                    .attr("style")
                    .and_then(|s| css_color(&s))
                    .or_else(|| token.attr("color").and_then(|c| parse_color(&c)))
            }
            ("a", _) => {
                link = token.attr("href").map(|dest| Link { dest, title: token.attr("title").unwrap_or_default() })
            }
            _ => {
                self.dim(raw, source);
                return;
            }
        }
        match token.kind {
            TagKind::Open => {
                self.stack.push(Frame::Html { name: name.to_string(), style, link });
                self.open_html.push((name.to_string(), source.start));
            }
            TagKind::Close => {
                if let Some(p) = self.stack.iter().rposition(|f| matches!(f, Frame::Html { name: n, .. } if n == name))
                {
                    self.stack.remove(p);
                }
                self.close_html(name, source.end);
            }
            TagKind::SelfClosing | TagKind::Comment => {}
        }
    }

    /// A closing tag ends the element its innermost opening tag of that name started.
    fn close_html(&mut self, name: &str, end: usize) {
        if let Some(p) = self.open_html.iter().rposition(|(n, _)| n == name) {
            let (_, start) = self.open_html.remove(p);
            self.elements.push(start..end);
        }
    }

    fn dim(&mut self, raw: &str, source: Range<usize>) {
        let mut style = self.style();
        style.dim = true;
        self.push_text(raw.to_string(), style, SourceMap::new(raw.len(), source));
    }
}

/// A text run split at emoji shortcodes (replaced by the emoji) and, outside
/// links, bare URLs (made links).
fn split_specials(text: &str, map: &SourceMap, style: Style, link: Option<Link>) -> Vec<Inline> {
    let mut out = Vec::new();
    let push = |text: &str, map: SourceMap, link: Option<Link>, out: &mut Vec<Inline>| {
        if !text.is_empty() {
            out.push(Inline::Text { text: text.to_string(), style, link, map });
        }
    };
    let mut pos = 0;
    for (range, kind) in specials(text, link.is_none()) {
        if range.start > pos {
            push(&text[pos..range.start], map.slice(pos..range.start), link.clone(), &mut out);
        }
        let piece = map.slice(range.clone());
        match kind {
            Special::Emoji(e) => push(e, collapse(piece, e.len()), link.clone(), &mut out),
            Special::Url(dest) => {
                push(&text[range.clone()], piece, Some(Link { dest, title: String::new() }), &mut out)
            }
        }
        pos = range.end;
    }
    if pos < text.len() {
        push(&text[pos..], map.slice(pos..text.len()), link, &mut out);
    }
    out
}

/// Text runs with what the editor recognizes in text: emoji shortcodes,
/// bare URLs (outside links) and intraword sub/superscript.
pub fn recognize(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, style, link, map } if !style.code && !style.dim => {
                out.extend(split_specials(&text, &map, style, link))
            }
            other => out.push(other),
        }
    }
    sub_sup(out, &mut Vec::new())
}

/// A map for a replacement text of `len` bytes standing for the whole source of `map`.
fn collapse(map: SourceMap, len: usize) -> SourceMap {
    let source = map.source_range();
    SourceMap { pieces: vec![(0, source)], len }
}

/// The inlines of an HTML block's events.
fn html_inlines(doc: &Document, events: &Events) -> Vec<Inline> {
    let mut b = InlineBuilder { out: Vec::new(), stack: Vec::new(), elements: Vec::new(), open_html: Vec::new() };
    // One HTML text over the whole block, so that elements spanning lines work.
    let Some(first) = events.first() else { return Vec::new() };
    let start = first.1.start;
    let end = events.last().map_or(start, |e| e.1.end);
    let html = doc.text[start..end].trim_end_matches('\n').to_string();
    b.html(&html, start..start + html.len());
    // Line breaks of the source don't break the rendered text.
    b.out
        .into_iter()
        .filter_map(|i| match i {
            Inline::Text { text, style, link, map } => {
                let t = text.trim_matches('\n');
                if t.trim().is_empty() && !style.dim {
                    return None;
                }
                Some(Inline::Text { text: t.replace('\n', " "), style, link, map })
            }
            other => Some(other),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagKind {
    Open,
    Close,
    SelfClosing,
    Comment,
}

#[derive(Debug, Clone)]
struct HtmlToken {
    range: Range<usize>,
    name: String,
    kind: TagKind,
    attrs: Vec<(String, String)>,
}

impl HtmlToken {
    fn attr(&self, name: &str) -> Option<String> {
        self.attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone())
    }
}

/// The tags and comments in HTML text.
fn html_tokens(html: &str) -> Vec<HtmlToken> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    while let Some(lt) = html[i..].find('<').map(|p| p + i) {
        if html[lt..].starts_with("<!--") {
            let end = html[lt..].find("-->").map_or(html.len(), |e| lt + e + 3);
            out.push(HtmlToken { range: lt..end, name: String::new(), kind: TagKind::Comment, attrs: Vec::new() });
            i = end;
            continue;
        }
        // The end of the tag, outside quoted attribute values.
        let mut j = lt + 1;
        let mut quote = None;
        while j < bytes.len() {
            match (quote, bytes[j]) {
                (None, b'"' | b'\'') => quote = Some(bytes[j]),
                (Some(q), c) if c == q => quote = None,
                (None, b'>') => break,
                _ => {}
            }
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }
        let inner = &html[lt + 1..j];
        let (kind, inner) = if let Some(rest) = inner.strip_prefix('/') {
            (TagKind::Close, rest)
        } else if let Some(rest) = inner.strip_suffix('/') {
            (TagKind::SelfClosing, rest)
        } else {
            (TagKind::Open, inner)
        };
        let name_end = inner.find(|c: char| c.is_whitespace()).unwrap_or(inner.len());
        let name = inner[..name_end].to_ascii_lowercase();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            i = lt + 1;
            continue;
        }
        let attrs = parse_attrs(&inner[name_end..]);
        out.push(HtmlToken { range: lt..j + 1, name, kind, attrs });
        i = j + 1;
    }
    out
}

fn parse_attrs(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = s.trim_start();
    while !rest.is_empty() {
        let name_end = rest.find(|c: char| c.is_whitespace() || c == '=').unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        rest = rest[name_end..].trim_start();
        let mut value = String::new();
        if let Some(r) = rest.strip_prefix('=') {
            let r = r.trim_start();
            if let Some(q @ ('"' | '\'')) = r.chars().next() {
                let end = r[1..].find(q).map_or(r.len(), |e| e + 1);
                value = r[1..end].to_string();
                rest = r.get(end + 1..).unwrap_or("").trim_start();
            } else {
                let end = r.find(char::is_whitespace).unwrap_or(r.len());
                value = r[..end].to_string();
                rest = r[end..].trim_start();
            }
        }
        if name.is_empty() {
            break;
        }
        out.push((name, value));
    }
    out
}

/// The `color` of a CSS declaration list.
fn css_color(style: &str) -> Option<Rgb> {
    style.split(';').find_map(|decl| {
        let (prop, value) = decl.split_once(':')?;
        (prop.trim().eq_ignore_ascii_case("color")).then(|| parse_color(value.trim()))?
    })
}

/// A CSS color: `#rgb`, `#rrggbb`, `rgb(r, g, b)` or a common name.
pub fn parse_color(value: &str) -> Option<Rgb> {
    let v = value.trim().to_ascii_lowercase();
    if let Some(hex) = v.strip_prefix('#') {
        let digit = |s: &str| u8::from_str_radix(s, 16).ok();
        return match hex.len() {
            3 => {
                let c: Vec<u8> = hex.chars().filter_map(|c| digit(&c.to_string())).map(|d| d * 17).collect();
                (c.len() == 3).then(|| Rgb(c[0], c[1], c[2]))
            }
            6 => Some(Rgb(digit(&hex[0..2])?, digit(&hex[2..4])?, digit(&hex[4..6])?)),
            _ => None,
        };
    }
    if let Some(args) = v.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        let c: Vec<u8> = args.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        return (c.len() == 3).then(|| Rgb(c[0], c[1], c[2]));
    }
    Some(match v.as_str() {
        "black" => Rgb(0, 0, 0),
        "white" => Rgb(255, 255, 255),
        "red" => Rgb(255, 0, 0),
        "green" => Rgb(0, 128, 0),
        "blue" => Rgb(0, 0, 255),
        "yellow" => Rgb(255, 255, 0),
        "orange" => Rgb(255, 165, 0),
        "purple" => Rgb(128, 0, 128),
        "gray" | "grey" => Rgb(128, 128, 128),
        "silver" => Rgb(192, 192, 192),
        "brown" => Rgb(165, 42, 42),
        "pink" => Rgb(255, 192, 203),
        "teal" => Rgb(0, 128, 128),
        "navy" => Rgb(0, 0, 128),
        "maroon" => Rgb(128, 0, 0),
        "olive" => Rgb(128, 128, 0),
        "lime" => Rgb(0, 255, 0),
        "aqua" | "cyan" => Rgb(0, 255, 255),
        "fuchsia" | "magenta" => Rgb(255, 0, 255),
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Special {
    Emoji(&'static str),
    Url(String),
}

/// Emoji shortcodes and (when `urls`) bare URLs in a text.
fn specials(text: &str, urls: bool) -> Vec<(Range<usize>, Special)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &text[i..];
        if bytes[i] == b':'
            && let Some(len) = rest[1..].find(':')
            && len > 0
            && rest[1..1 + len].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-'))
            && let Some(emoji) = emojis::get_by_shortcode(&rest[1..1 + len])
        {
            out.push((i..i + len + 2, Special::Emoji(emoji.as_str())));
            i += len + 2;
            continue;
        }
        let at_word_start = i == 0 || !text[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric());
        if urls
            && at_word_start
            && (rest.starts_with("https://") || rest.starts_with("http://") || rest.starts_with("www."))
        {
            let len = url_length(rest);
            let min = if rest.starts_with("www.") { 5 } else { rest.find("//").unwrap() + 3 };
            if len >= min {
                let url = &rest[..len];
                let dest = if url.starts_with("www.") { format!("http://{url}") } else { url.to_string() };
                out.push((i..i + len, Special::Url(dest)));
                i += len;
                continue;
            }
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// The length of the URL at the start of `s`, without trailing punctuation
/// (and a closing parenthesis it doesn't open).
fn url_length(s: &str) -> usize {
    let mut end = s.find(|c: char| c.is_whitespace() || c == '<' || !c.is_ascii()).unwrap_or(s.len());
    loop {
        let url = &s[..end];
        let Some(last) = url.chars().last() else { return 0 };
        let unbalanced_paren = last == ')' && url.matches('(').count() < url.matches(')').count();
        if matches!(last, '.' | ',' | ':' | ';' | '!' | '?' | '\'' | '"' | '*' | '_' | '~') || unbalanced_paren {
            end -= 1;
        } else {
            return end;
        }
    }
}

/// Applies `==highlight==`: pairs of `==` in the text runs (not in code)
/// highlight what is between them and disappear.
fn highlight(inlines: Vec<Inline>, elements: &mut Vec<Range<usize>>) -> Vec<Inline> {
    // The positions of `==` as (run, offset).
    let mut marks: Vec<(usize, usize)> = Vec::new();
    for (i, inline) in inlines.iter().enumerate() {
        if let Inline::Text { text, style, .. } = inline
            && !style.code
            && !style.dim
        {
            let mut from = 0;
            while let Some(p) = text[from..].find("==").map(|p| p + from) {
                marks.push((i, p));
                from = p + 2;
            }
        }
    }
    if marks.len() < 2 {
        return inlines;
    }
    let pairs = marks.len() / 2;
    let marks = &marks[..pairs * 2];
    let source_of = |(run, p): (usize, usize)| match &inlines[run] {
        Inline::Text { map, .. } => map.source_of(p)..map.source_of(p + 2),
        _ => 0..0,
    };
    for pair in marks.chunks(2) {
        elements.push(source_of(pair[0]).start..source_of(pair[1]).end);
    }
    let mut out = Vec::new();
    let mut on = false;
    let mut next = 0;
    for (i, inline) in inlines.into_iter().enumerate() {
        let Inline::Text { text, style, link, map } = inline else {
            out.push(inline);
            continue;
        };
        let mut pos = 0;
        while next < marks.len() && marks[next].0 == i {
            let p = marks[next].1;
            if p > pos {
                let mut s = style;
                s.highlight |= on;
                out.push(Inline::Text {
                    text: text[pos..p].to_string(),
                    style: s,
                    link: link.clone(),
                    map: map.slice(pos..p),
                });
            }
            on = !on;
            pos = p + 2;
            next += 1;
        }
        if pos < text.len() {
            let mut s = style;
            s.highlight |= on;
            out.push(Inline::Text { text: text[pos..].to_string(), style: s, link, map: map.slice(pos..text.len()) });
        }
    }
    out
}

/// Typora's `~sub~` and `^sup^` inside words (`H~2~O`, `x^2^`), which the
/// parser leaves as text: a delimiter, text without spaces, the delimiter.
fn sub_sup(inlines: Vec<Inline>, elements: &mut Vec<Range<usize>>) -> Vec<Inline> {
    let mut out = Vec::with_capacity(inlines.len());
    for inline in inlines {
        let Inline::Text { text, style, link, map } = inline else {
            out.push(inline);
            continue;
        };
        if style.code || style.dim || !text.contains(['~', '^']) {
            out.push(Inline::Text { text, style, link, map });
            continue;
        }
        let mut pos = 0;
        let mut i = 0;
        let bytes = text.as_bytes();
        while i < bytes.len() {
            let d = bytes[i];
            let single = |at: usize| {
                bytes.get(at) == Some(&d) && (at == 0 || bytes[at - 1] != d) && bytes.get(at + 1) != Some(&d)
            };
            if (d == b'~' || d == b'^') && single(i) {
                let close = text[i + 1..].find(d as char).map(|e| i + 1 + e);
                if let Some(close) = close
                    && close > i + 1
                    && single(close)
                    && !text[i + 1..close].contains(char::is_whitespace)
                {
                    if i > pos {
                        out.push(Inline::Text {
                            text: text[pos..i].to_string(),
                            style,
                            link: link.clone(),
                            map: map.slice(pos..i),
                        });
                    }
                    elements.push(map.source_of(i)..map.source_of(close + 1));
                    let mut s = style;
                    if d == b'~' {
                        s.sub = true;
                    } else {
                        s.sup = true;
                    }
                    out.push(Inline::Text {
                        text: text[i + 1..close].to_string(),
                        style: s,
                        link: link.clone(),
                        map: map.slice(i + 1..close),
                    });
                    pos = close + 1;
                    i = close + 1;
                    continue;
                }
            }
            i += 1;
        }
        if pos < text.len() {
            out.push(Inline::Text { text: text[pos..].to_string(), style, link, map: map.slice(pos..text.len()) });
        }
    }
    out
}

/// The text of inlines as shown, for counting words and plain-text copies.
pub fn shown_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text { text, style, .. } if !style.dim => out.push_str(text),
            Inline::Break { .. } => out.push('\n'),
            Inline::Math { tex, .. } => out.push_str(tex),
            Inline::Image { alt, .. } => out.push_str(alt),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(text: &str) -> Vec<Inline> {
        let doc = Document::parse(text);
        inlines(&doc, doc.block_events(0))
    }

    /// (text, style flags) of the text runs.
    fn texts(text: &str) -> Vec<(String, String)> {
        runs(text)
            .into_iter()
            .filter_map(|i| match i {
                Inline::Text { text, style, link, .. } => {
                    let mut f = String::new();
                    for (on, c) in [
                        (style.bold, 'b'),
                        (style.italic, 'i'),
                        (style.strike, 's'),
                        (style.underline, 'u'),
                        (style.code, 'c'),
                        (style.highlight, 'h'),
                        (style.sub, '_'),
                        (style.sup, '^'),
                        (style.dim, 'd'),
                        (link.is_some(), 'l'),
                        (style.color.is_some(), 'C'),
                    ] {
                        if on {
                            f.push(c);
                        }
                    }
                    Some((text, f))
                }
                _ => None,
            })
            .collect()
    }

    fn t(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn emphasis_and_friends() {
        assert_eq!(
            texts("a *i* **b** ~~s~~ `c` ~sub~ ^sup^"),
            t(&[
                ("a ", ""),
                ("i", "i"),
                (" ", ""),
                ("b", "b"),
                (" ", ""),
                ("s", "s"),
                (" ", ""),
                ("c", "c"),
                (" ", ""),
                ("sub", "_"),
                (" ", ""),
                ("sup", "^")
            ])
        );
    }

    #[test]
    fn sub_and_superscript_inside_words() {
        assert_eq!(texts("H~2~O and x^2^"), t(&[("H", ""), ("2", "_"), ("O and x", ""), ("2", "^")]));
        assert_eq!(texts("a ~ b ~ c"), t(&[("a ~ b ~ c", "")]));
    }

    #[test]
    fn highlight_pairs() {
        assert_eq!(texts("x ==hi== y == z"), t(&[("x ", ""), ("hi", "h"), (" y == z", "")]));
        assert_eq!(texts("`==c==`"), t(&[("==c==", "c")]));
        assert_eq!(texts("==a **b** c=="), t(&[("a ", "h"), ("b", "bh"), (" c", "h")]));
    }

    #[test]
    fn emoji_shortcodes() {
        assert_eq!(texts("I :smile: :not-an-emoji:"), t(&[("I ", ""), ("😄", ""), (" :not-an-emoji:", "")]));
    }

    #[test]
    fn bare_urls() {
        assert_eq!(
            texts("see https://example.com/a_(b). and www.x.org!"),
            t(&[("see ", ""), ("https://example.com/a_(b)", "l"), (". and ", ""), ("www.x.org", "l"), ("!", "")])
        );
        let link = runs("go www.x.org").into_iter().find_map(|i| match i {
            Inline::Text { link: Some(l), .. } => Some(l.dest),
            _ => None,
        });
        assert_eq!(link.as_deref(), Some("http://www.x.org"));
        // Not inside a link's text.
        assert_eq!(texts("[https://a.b](x)"), t(&[("https://a.b", "l")]));
    }

    #[test]
    fn links_of_every_kind() {
        let dests = |text: &str| -> Vec<String> {
            runs(text)
                .into_iter()
                .filter_map(|i| match i {
                    Inline::Text { link: Some(l), .. } => Some(l.dest),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(dests("[a](b.md \"t\")"), vec!["b.md"]);
        assert_eq!(dests("[a][id]\n\n[id]: http://x.y"), vec!["http://x.y"]);
        assert_eq!(dests("<http://auto.link>"), vec!["http://auto.link"]);
        assert_eq!(dests("[h](#head)"), vec!["#head"]);
    }

    #[test]
    fn inline_html() {
        assert_eq!(
            texts("<u>u</u> <span style=\"color:red\">r</span> <x-y>z</x-y>"),
            t(&[("u", "u"), (" ", ""), ("r", "C"), (" ", ""), ("<x-y>", "d"), ("z", ""), ("</x-y>", "d")])
        );
        let r = runs("a<br>b <img src=\"p.png\" width=\"100\">");
        assert!(matches!(r[1], Inline::Break { .. }));
        assert!(matches!(&r[3], Inline::Image { src, width: Some(w), .. } if src == "p.png" && *w == 100.0));
    }

    #[test]
    fn embeds_are_placeholders() {
        let doc = Document::parse("<iframe src=\"https://www.youtube.com/embed/x\"></iframe>\n");
        let n = nodes(&doc, doc.block_events(0));
        assert_eq!(n.len(), 1);
        let Node::Html(inl) = &n[0] else { panic!("{n:?}") };
        assert!(
            matches!(&inl[..], [Inline::Embed { tag, src: Some(s), .. }] if tag == "iframe" && s.contains("youtube"))
        );
    }

    #[test]
    fn colors() {
        assert_eq!(parse_color("#f00"), Some(Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff80"), Some(Rgb(0, 255, 128)));
        assert_eq!(parse_color("rgb(1, 2, 3)"), Some(Rgb(1, 2, 3)));
        assert_eq!(parse_color("Red"), Some(Rgb(255, 0, 0)));
        assert_eq!(parse_color("nope"), None);
    }

    #[test]
    fn math_images_footnotes_and_breaks() {
        let r = runs("a $x^2$ ![alt *t*](p.png \"T\")[^n]\nb\n\n[^n]: note");
        assert!(matches!(&r[1], Inline::Math { tex, display: false, .. } if tex == "x^2"));
        assert!(
            matches!(&r[3], Inline::Image { src, alt, title, .. } if src == "p.png" && alt == "alt t" && title == "T")
        );
        assert!(matches!(&r[4], Inline::FootnoteRef { number: 1, .. }));
        assert!(matches!(r[5], Inline::Break { .. }));
    }

    #[test]
    fn runs_map_back_to_the_source() {
        let text = "ab **cd** [ef](x) :smile: g\\*h";
        let doc = Document::parse(text);
        let r = inlines(&doc, doc.block_events(0));
        let find = |s: &str| {
            r.iter()
                .find_map(|i| match i {
                    Inline::Text { text: t, map, .. } if t == s => Some(map.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let cd = find("cd");
        assert_eq!(&text[cd.source_range()], "cd");
        assert_eq!(cd.source_of(1), text.find("cd").unwrap() + 1);
        let smile = find("😄");
        assert_eq!(&text[smile.source_range()], ":smile:");
        // The escape: `\*` shows as `*`.
        let tail = r.iter().rev().find_map(|i| match i {
            Inline::Text { text: t, map, .. } => Some((t.clone(), map.clone())),
            _ => None,
        });
        let (t, map) = tail.unwrap();
        assert!(t.ends_with("g*h"), "{t}");
        assert_eq!(map.source_of(t.len()), text.len());
    }

    #[test]
    fn nested_nodes() {
        let doc = Document::parse("> # H\n> - a\n>   - b\n> ```\n> code\n> ```\n");
        // A block's events leave out its own start: these are the quote's children.
        let children = nodes(&doc, doc.block_events(0));
        assert!(matches!(&children[0], Node::Heading(1, _)));
        let Node::List { items, .. } = &children[1] else { panic!() };
        assert!(matches!(&items[0].children[1], Node::List { .. }));
        assert!(matches!(&children[2], Node::Code { text, .. } if text == "code\n"));
    }

    #[test]
    fn table_cells() {
        let doc = Document::parse("| a | **b** |\n|:--|--:|\n| 1 | 2 |\n");
        let t = table_block(&doc, 0);
        assert_eq!(t.align, vec![Alignment::Left, Alignment::Right]);
        assert_eq!(t.header.len(), 2);
        assert_eq!(t.rows.len(), 1);
        assert!(matches!(&t.header[1][0], Inline::Text { text, style, .. } if text == "b" && style.bold));
    }

    #[test]
    fn shown_text_of_a_paragraph() {
        let doc = Document::parse("a **b** <u>c</u>\nd");
        assert_eq!(shown_text(&inlines(&doc, doc.block_events(0))), "a b c\nd");
    }
}
