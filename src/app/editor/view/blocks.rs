//! Rendered blocks: paragraphs, headings, list items, quotes, code, math,
//! diagrams, images and tables.

use super::hit::{HitInfo, HitRun};
use super::inline::{inline_text, paragraph_view};
use super::theme::{
    BLOCK_GAP, CODE_FONT, CODE_PADDING, HEADING_SCALE, LINE_HEIGHT, LIST_INDENT, Palette, QUOTE_BAR, QUOTE_PADDING,
    bold,
};
use super::{Ctx, SpanLink, TextBase};
use crate::app::editor::highlight::{self};
use crate::app::editor::render::{RenderKey, Rendered};
use crate::app::editor::{Editor, EditorMsg};
use crate::app::view::APP_FONT;
use crate::markdown::content::{self, Inline, Node, TableNode};
use crate::markdown::{BlockKind, ListItem, images};
use iced::advanced::text::Wrapping;
use iced::widget::text::Span;
use iced::widget::{
    Column, Row, Stack, button, column, container, image, mouse_area, rich_text, row, rule, space, svg, text,
};
use iced::{Alignment, Color, Element, Fill, Font, Length, border};
use std::sync::Arc;

pub(super) fn quote_view<'a>(content: Element<'a, EditorMsg>, p: Palette) -> Element<'a, EditorMsg> {
    row![
        container(space()).width(QUOTE_BAR).height(Fill).style(move |_| container::background(p.quote_bar)),
        container(content).padding(iced::Padding { left: QUOTE_PADDING - QUOTE_BAR, ..Default::default() }),
    ]
    .height(Length::Shrink)
    .into()
}

/// Nested content: paragraphs, headings, lists, quotes, code...
fn nodes_view<'a>(nodes: &[Node], ctx: &Ctx<'a>, width: f32, block: usize) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let base = TextBase { size: ctx.size(1.0), font: APP_FONT, color: p.text };
    let mut col = Column::new().spacing(BLOCK_GAP * ctx.zoom * 0.6);
    for node in nodes {
        col = col.push(match node {
            Node::Paragraph(inlines) | Node::Html(inlines) => paragraph_view(inlines, base, ctx, width, block),
            Node::Heading(level, inlines) => {
                let scale = HEADING_SCALE[usize::from((*level).clamp(1, 6)) - 1];
                let b = TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: p.heading };
                inline_text(inlines, b, ctx, Some(width), block)
            }
            Node::Code { lang, text: code } => code_text_view(code, lang.as_deref(), None, ctx, width, block),
            Node::Math(tex) => math_view(tex, true, ctx),
            Node::Quote(children) => quote_view(nodes_view(children, ctx, width - QUOTE_PADDING, block), p),
            Node::List { start, items } => {
                let mut list = Column::new().spacing(2);
                for (n, item) in items.iter().enumerate() {
                    let marker_w = LIST_INDENT * ctx.zoom;
                    let marker: Element<EditorMsg> = match (item.task, start) {
                        (Some(c), _) => text(if c { "☑" } else { "☐" }).size(ctx.size(1.0)).width(marker_w).into(),
                        (None, Some(s)) => {
                            text(format!("{}.", s + n as u64)).size(ctx.size(1.0)).width(marker_w).into()
                        }
                        (None, None) => text("•").size(ctx.size(1.0)).width(marker_w).into(),
                    };
                    list = list.push(row![marker, nodes_view(&item.children, ctx, width - marker_w, block)]);
                }
                list.into()
            }
            Node::Rule => rule::horizontal(1).into(),
            Node::Table(t) => table_view(t, ctx, block),
            Node::FrontMatter(t) => text(t.clone()).font(CODE_FONT).color(p.muted).into(),
            Node::Footnote { number, children } => {
                row![text(format!("[{number}]")).color(p.muted), nodes_view(children, ctx, width - 32.0, block)].into()
            }
        });
    }
    col.into()
}

fn code_view<'a>(
    doc: &crate::markdown::Document,
    block: usize,
    lang: Option<&str>,
    ctx: &Ctx<'a>,
    width: f32,
) -> Element<'a, EditorMsg> {
    // The code with where each line starts in the source.
    let mut code = String::new();
    let mut starts = Vec::new();
    for (e, r) in doc.block_events(block) {
        if let pulldown_cmark::Event::Text(t) = e {
            let mut offset = r.start;
            for line in t.split_inclusive('\n') {
                starts.push(offset);
                offset += line.len();
            }
            code.push_str(t);
        }
    }
    code_text_view(&code, lang, Some(&starts), ctx, width, block)
}

/// Code in a box, highlighted for its language; `starts` are the source
/// offsets of its lines, when they are known.
fn code_text_view<'a>(
    code: &str,
    lang: Option<&str>,
    starts: Option<&[usize]>,
    ctx: &Ctx<'a>,
    width: f32,
    block: usize,
) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let code = code.strip_suffix('\n').unwrap_or(code);
    let size = ctx.size(0.9);
    let colored = highlight::highlight_code(code, lang, p.dark);
    let mut lines = Column::new();
    for (n, line) in code.split('\n').enumerate() {
        let pieces: Vec<(String, Color)> =
            colored.as_ref().and_then(|c| c.get(n).cloned()).unwrap_or_else(|| vec![(line.to_string(), p.text)]);
        let spans: Vec<Span<'static, SpanLink, Font>> =
            pieces.iter().map(|(t, c)| Span::new(t.clone()).color(*c).font(CODE_FONT).size(size)).collect();
        let rich = rich_text(spans).size(size).font(CODE_FONT).line_height(LINE_HEIGHT).wrapping(Wrapping::WordOrGlyph);
        let element: Element<EditorMsg> = match starts.and_then(|s| s.get(n)) {
            Some(&start) => {
                let map = content::source_map(line.len(), start..start + line.len());
                let info = Arc::new(HitInfo {
                    block,
                    runs: vec![HitRun { text: line.to_string(), size, font: CODE_FONT, map: Some(map) }],
                    width: width - 2.0 * CODE_PADDING,
                });
                mouse_area(rich.width(width - 2.0 * CODE_PADDING))
                    .on_move(move |pt| EditorMsg::PointerOver(info.clone(), pt))
                    .on_press(EditorMsg::TextPressed)
                    .into()
            }
            None => rich.into(),
        };
        lines =
            lines.push(if line.is_empty() { text(" ").size(size).line_height(LINE_HEIGHT).into() } else { element });
    }
    container(lines)
        .padding(CODE_PADDING)
        .width(Fill)
        .style(move |_| container::background(p.code_bg).border(border::rounded(4)))
        .into()
}

pub(super) fn math_view<'a>(tex: &str, display: bool, ctx: &Ctx<'a>) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let key = RenderKey::Math { tex: tex.trim().to_string(), display, dark: p.dark };
    let rendered = ctx.renders.get(&key);
    let content: Element<EditorMsg> = match rendered {
        Some(Rendered::Done(Ok(s))) => {
            // Math is typeset at 12 pt (16 px); the body text is BODY_SIZE.
            let scale = ctx.size(1.0) / 16.0 * if display { 1.15 } else { 1.0 };
            svg(s.handle.clone()).width(s.width * scale).height(s.height * scale).into()
        }
        Some(Rendered::Done(Err(e))) => {
            let source = text(if display { tex.to_string() } else { format!("${tex}$") })
                .font(CODE_FONT)
                .size(ctx.size(0.9))
                .color(p.math);
            if display {
                column![source, text(e.clone()).size(ctx.size(0.8)).color(p.error)].spacing(4).into()
            } else {
                source.into()
            }
        }
        _ => text(tex.to_string()).font(CODE_FONT).size(ctx.size(0.9)).color(p.muted).into(),
    };
    if display { container(content).center_x(Fill).padding([4, 0]).into() } else { content }
}

pub(super) fn mermaid_view<'a>(source: &str, ctx: &Ctx<'a>, width: f32) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let key = RenderKey::Mermaid(source.to_string());
    match ctx.renders.get(&key) {
        Some(Rendered::Done(Ok(s))) => {
            let scale = ctx.zoom.min(width / s.width.max(1.0));
            container(svg(s.handle.clone()).width(s.width * scale).height(s.height * scale)).center_x(Fill).into()
        }
        Some(Rendered::Done(Err(e))) => column![
            container(text(source.to_string()).font(CODE_FONT).size(ctx.size(0.9)).color(p.text))
                .padding(CODE_PADDING)
                .width(Fill)
                .style(move |_| container::background(p.code_bg).border(border::rounded(4))),
            text(e.clone()).size(ctx.size(0.8)).color(p.error),
        ]
        .spacing(4)
        .into(),
        _ => text("…").color(p.muted).into(),
    }
}

/// An image of the page: the file, or its alt text when it can't be shown.
pub(super) fn image_view<'a>(inline: &Inline, ctx: &Ctx<'a>, width: f32, block: usize) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let Inline::Image { src, alt, width: w, source, .. } = inline else { return space().into() };
    let editor = ctx.editor;
    let at = source.start;
    let path = images::image_path(editor.page.path(), editor.page.text(), src);
    let max = w.map_or(width, |w| (w * ctx.zoom).min(width));
    let shown: Element<EditorMsg> = match path {
        Some(path) if path.is_file() => {
            let is_svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
            if is_svg {
                svg(svg::Handle::from_path(&path)).width(Length::Shrink).height(Length::Shrink).into()
            } else {
                let img = image(image::Handle::from_path(&path));
                match w {
                    Some(_) => img.width(max).into(),
                    None => container(img).max_width(max).into(),
                }
            }
        }
        Some(_) => text(format!("🖼 {} (missing: {src})", alt)).size(ctx.size(0.9)).color(p.error).into(),
        None => rich_text(vec![
            Span::new(format!("🖼 {} ", if alt.is_empty() { "image" } else { alt })).color(p.muted),
            Span::new(src.clone()).color(p.link).underline(true).link(SpanLink { dest: src.clone(), at }),
        ])
        .size(ctx.size(0.9))
        .on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at))
        .into(),
    };
    let _ = block;
    mouse_area(shown).on_press(EditorMsg::PressedAt(at)).into()
}

pub(super) fn embed_view<'a>(tag: &str, src: Option<&str>, p: Palette, ctx: &Ctx<'a>) -> Element<'a, EditorMsg> {
    let label = format!("▶ {tag}");
    let mut spans: Vec<Span<'static, SpanLink, Font>> = vec![Span::new(label).color(p.muted)];
    if let Some(src) = src {
        spans.push(
            Span::new(format!(" {src}")).color(p.link).underline(true).link(SpanLink { dest: src.to_string(), at: 0 }),
        );
    }
    container(rich_text(spans).size(ctx.size(0.9)).on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at)))
        .padding(12)
        .style(move |_| container::background(p.code_bg).border(border::color(p.border).width(1).rounded(4)))
        .into()
}

/// A table that is not being edited.
fn table_view<'a>(t: &TableNode, ctx: &Ctx<'a>, block: usize) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let columns = t.header.len().max(1);
    let mut grid = Column::new();
    let all_rows = std::iter::once(&t.header).chain(t.rows.iter());
    for (r, cells) in all_rows.enumerate() {
        let mut contents = Row::new();
        let mut frames = Row::new();
        for c in 0..columns {
            let inlines = cells.get(c).cloned().unwrap_or_default();
            let header = r == 0;
            let base =
                TextBase { size: ctx.size(1.0), font: if header { bold(APP_FONT) } else { APP_FONT }, color: p.text };
            let align_x = match t.align.get(c) {
                Some(pulldown_cmark::Alignment::Center) => Alignment::Center,
                Some(pulldown_cmark::Alignment::Right) => Alignment::End,
                _ => Alignment::Start,
            };
            contents = contents.push(
                container(inline_text(&inlines, base, ctx, None, block))
                    .width(Length::FillPortion(1))
                    .padding([4, 8])
                    .align_x(align_x),
            );
            // The whole cell (beside and below its text too) takes the clicks.
            frames = frames.push(
                mouse_area(cell_frame(
                    Length::FillPortion(1),
                    if header { p.header_bg } else { Color::TRANSPARENT },
                    p,
                ))
                .on_press(EditorMsg::TableCellPressed(block, r, c)),
            );
        }
        grid = grid.push(table_row(contents, frames));
    }
    grid.into()
}

/// A table row: the cells' contents over their frames (background and
/// border), which take the height of the row's tallest cell so that the
/// borders of a row line up when a cell wraps.
pub(super) fn table_row<'a>(contents: Row<'a, EditorMsg>, frames: Row<'a, EditorMsg>) -> Element<'a, EditorMsg> {
    Stack::new().push(contents).push_under(frames).into()
}

/// A table cell's background and border, as tall as its row.
pub(super) fn cell_frame<'a>(width: impl Into<Length>, bg: Color, p: Palette) -> Element<'a, EditorMsg> {
    container(space())
        .width(width)
        .height(Fill)
        .style(move |_| container::background(bg).border(border::color(p.border).width(1)))
        .into()
}

impl Editor {
    pub(super) fn block_view<'a>(&'a self, i: usize, ctx: &Ctx<'a>, width: f32) -> Element<'a, EditorMsg> {
        let doc = self.page.doc();
        let block = &doc.blocks[i];
        let p = ctx.p;
        let base = TextBase { size: ctx.size(1.0), font: APP_FONT, color: p.text };
        match &block.kind {
            BlockKind::Heading(level) => {
                let scale = HEADING_SCALE[usize::from((*level).clamp(1, 6)) - 1];
                let inlines = content::inlines(doc, doc.block_events(i));
                let base = TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: p.heading };
                let heading = inline_text(&inlines, base, ctx, Some(width), i);
                if *level <= 2 {
                    column![
                        heading,
                        rule::horizontal(1).style(move |_| rule::Style {
                            color: p.border,
                            radius: 0.0.into(),
                            fill_mode: rule::FillMode::Full,
                            snap: true,
                        })
                    ]
                    .spacing(4)
                    .into()
                } else {
                    heading
                }
            }
            BlockKind::ThematicBreak => container(rule::horizontal(2).style(move |_| rule::Style {
                color: p.border,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
                snap: true,
            }))
            .padding([8, 0])
            .into(),
            BlockKind::Code { lang, .. } => {
                if block.kind.is_mermaid() {
                    return mermaid_view(&crate::app::editor::render::code_of(doc, i), ctx, width);
                }
                code_view(doc, i, lang.as_deref(), ctx, width)
            }
            BlockKind::Math => {
                let tex = content::nodes(doc, doc.block_events(i))
                    .into_iter()
                    .find_map(|n| match n {
                        Node::Math(t) => Some(t),
                        Node::Paragraph(inl) => inl.into_iter().find_map(|x| match x {
                            Inline::Math { tex, .. } => Some(tex),
                            _ => None,
                        }),
                        _ => None,
                    })
                    .unwrap_or_default();
                math_view(&tex, true, ctx)
            }
            BlockKind::Table => table_view(&content::table_block(doc, i), ctx, i),
            BlockKind::FrontMatter => {
                let source = &self.page.text()[block.range.clone()];
                container(text(source.to_string()).font(CODE_FONT).size(ctx.size(0.85)).color(p.muted))
                    .padding(CODE_PADDING)
                    .width(Fill)
                    .style(move |_| container::background(p.code_bg).border(border::rounded(4)))
                    .into()
            }
            BlockKind::Toc => {
                let mut list = Column::new().spacing(2);
                for (h, info) in doc.headings.iter().enumerate() {
                    list = list.push(
                        button(text(info.text.clone()).size(ctx.size(0.95)).color(p.link))
                            .padding(iced::Padding {
                                left: 4.0 + 18.0 * f32::from(info.level - 1),
                                ..iced::Padding::from([1, 4])
                            })
                            .style(|_, _| button::Style::default())
                            .on_press(EditorMsg::HeadingClicked(h)),
                    );
                }
                list.into()
            }
            BlockKind::Other => {
                let source = &self.page.text()[block.range.clone()];
                text(source.to_string()).size(ctx.size(0.9)).color(p.muted).into()
            }
            BlockKind::ListItem(item) => self.list_item_view(i, item, ctx, width),
            BlockKind::Footnote(label) => {
                let number = doc.footnotes.get(label).copied().unwrap_or(0);
                let nodes = content::nodes(doc, doc.block_events(i));
                let label_w = 32.0 * ctx.zoom;
                row![
                    text(format!("[{number}]")).size(ctx.size(0.85)).color(p.muted).width(label_w),
                    nodes_view(&nodes, ctx, width - label_w, i),
                ]
                .into()
            }
            BlockKind::Quote | BlockKind::Html | BlockKind::Paragraph => {
                let nodes = content::nodes(doc, doc.block_events(i));
                if block.kind == BlockKind::Quote {
                    // An empty quote: a line of its bar.
                    let line = space().height(ctx.size(1.0) * LINE_HEIGHT);
                    quote_view(column![line, nodes_view(&nodes, ctx, width - QUOTE_PADDING - QUOTE_BAR, i)].into(), p)
                } else {
                    let _ = base;
                    nodes_view(&nodes, ctx, width, i)
                }
            }
        }
    }

    fn list_item_view<'a>(&'a self, i: usize, item: &ListItem, ctx: &Ctx<'a>, width: f32) -> Element<'a, EditorMsg> {
        let doc = self.page.doc();
        let p = ctx.p;
        let indent = LIST_INDENT * ctx.zoom * item.depth as f32;
        let marker_w = LIST_INDENT * ctx.zoom;
        let nodes = content::nodes(doc, doc.block_events(i));
        // A checked task's text is muted.
        let done = Ctx { p: Palette { text: p.muted, link: p.muted, heading: p.muted, ..p }, ..*ctx };
        let body_ctx = if item.task == Some(true) { &done } else { ctx };
        // An empty item keeps the height of a line of text.
        let body: Element<EditorMsg> = if nodes.is_empty() {
            space().height(ctx.size(1.0) * LINE_HEIGHT).into()
        } else {
            nodes_view(&nodes, body_ctx, width - indent - marker_w, i)
        };
        let marker: Element<EditorMsg> = if let Some(checked) = item.task {
            container(
                iced::widget::checkbox(checked).size(ctx.size(0.95)).on_toggle(move |_| EditorMsg::TaskToggled(i)),
            )
            .padding(iced::Padding { top: ctx.size(0.25), ..Default::default() })
            .width(marker_w)
            .into()
        } else {
            let glyph = match item.number {
                Some(n) => format!("{n}."),
                None => ["•", "◦", "▪"][item.depth.min(2)].to_string(),
            };
            text(glyph).size(ctx.size(1.0)).line_height(LINE_HEIGHT).color(p.text).width(marker_w).into()
        };
        row![space().width(indent), marker, body].into()
    }
}
