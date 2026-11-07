//! Rendered blocks: paragraphs, headings, list items, quotes, code, math,
//! diagrams, images and tables.

use super::heading;
use super::hit::{HitInfo, HitRun};
use super::inline::{cell_lines, inline_text, paragraph_view};
use super::rich::rich;
use super::table_widths::ColumnWidths;
use super::theme::{
    BLOCK_GAP, CELL_PADDING, CODE_FONT, CODE_PADDING, HEADING_SCALE, LINE_HEIGHT, LIST_INDENT, Palette, QUOTE_BAR,
    QUOTE_PADDING, bold,
};
use super::{Ctx, SpanLink, TextBase};
use crate::app::editor::highlight::{self};
use crate::app::editor::render::{RenderKey, Rendered};
use crate::app::editor::{DataHandle, Editor, EditorMsg, RightClick};
use crate::app::press_point::press_in;
use crate::app::view::APP_FONT;
use crate::markdown::content::{self, Inline, Node, TableNode};
use crate::markdown::{BlockKind, ListItem, images};
use iced::advanced::text::Wrapping;
use iced::widget::text::Span;
use iced::widget::{
    Column, Row, Stack, button, column, container, image, mouse_area, rich_text, row, rule, scrollable, space, svg,
    text,
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
                let level = (*level).clamp(1, 6);
                let b = TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: heading::text_color(level, p) };
                heading::framed(level, b.size, width, ctx.zoom, p, |w| inline_text(inlines, b, ctx, Some(w), block))
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
            Node::Table(t) => table_view(t, ctx, block, width),
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
    let colored = highlight::highlight_code_cached(code, lang, p.dark);
    let mut lines = Column::new();
    for (n, line) in code.split('\n').enumerate() {
        let plain;
        let pieces: &[(String, Color)] = match colored.as_ref().as_ref().and_then(|c| c.get(n)) {
            Some(pieces) => pieces,
            None => {
                plain = [(line.to_string(), p.text)];
                &plain
            }
        };
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
                let mut area = mouse_area(rich.width(width - 2.0 * CODE_PADDING))
                    .on_press(EditorMsg::TextPressed)
                    .on_right_press(EditorMsg::RightClicked(RightClick::Text));
                // As in other text: where a press lands, and where the pointer is
                // while an item of the assets pane is dragged.
                if ctx.editor.asset_dragging() {
                    let info = info.clone();
                    area = area.on_move(move |pt| EditorMsg::PointerOver(info.clone(), pt));
                }
                press_in(area, move |pt| EditorMsg::PointerOver(info.clone(), pt)).into()
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
    let max = w.map_or(width, |w| (w * ctx.zoom).min(width));
    let svg_view =
        |handle: svg::Handle| -> Element<EditorMsg> { svg(handle).width(Length::Shrink).height(Length::Shrink).into() };
    let image_view = |handle: image::Handle| -> Element<EditorMsg> {
        let img = image(handle);
        match w {
            Some(_) => img.width(max).into(),
            None => container(img).max_width(max).into(),
        }
    };
    let broken = |why: String| -> Element<EditorMsg> {
        text(format!("🖼 {alt} ({why})")).size(ctx.size(0.9)).color(p.error).into()
    };
    let shown: Element<EditorMsg> = match editor.data_image(src) {
        Some(DataHandle::Svg(handle)) => svg_view(handle),
        Some(DataHandle::Raster(handle)) => image_view(handle),
        Some(DataHandle::Invalid(e)) => broken(format!("invalid data URL: {e}")),
        None => match images::image_path(editor.page.path(), editor.page.text(), src) {
            Some(path) if editor.is_image_file(&path) => {
                if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
                    svg_view(svg::Handle::from_path(&path))
                } else {
                    image_view(image::Handle::from_path(&path))
                }
            }
            Some(_) => broken(format!("missing: {src}")),
            None => rich(vec![
                Span::new(format!("🖼 {} ", if alt.is_empty() { "image" } else { alt })).color(p.muted),
                Span::new(src.clone()).color(p.link).underline(true).link(SpanLink { dest: src.clone(), at }),
            ])
            .size(ctx.size(0.9))
            .on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at))
            .into(),
        },
    };
    let _ = block;
    mouse_area(shown)
        .on_press(EditorMsg::PressedAt(at))
        .on_right_press(EditorMsg::RightClicked(RightClick::At(at)))
        .into()
}

pub(super) fn embed_view<'a>(tag: &str, src: Option<&str>, p: Palette, ctx: &Ctx<'a>) -> Element<'a, EditorMsg> {
    let label = format!("▶ {tag}");
    let mut spans: Vec<Span<'static, SpanLink, Font>> = vec![Span::new(label).color(p.muted)];
    if let Some(src) = src {
        spans.push(
            Span::new(format!(" {src}")).color(p.link).underline(true).link(SpanLink { dest: src.to_string(), at: 0 }),
        );
    }
    container(rich(spans).size(ctx.size(0.9)).on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at)))
        .padding(12)
        .style(move |_| container::background(p.code_bg).border(border::color(p.border).width(1).rounded(4)))
        .into()
}

/// The base text of a table's row `r` (the header in bold).
pub(super) fn cell_base(r: usize, ctx: &Ctx) -> TextBase {
    TextBase { size: ctx.size(1.0), font: if r == 0 { bold(APP_FONT) } else { APP_FONT }, color: ctx.p.text }
}

/// The widths of the columns of a rendered table, in a page column of `width`.
pub(super) fn rendered_widths(t: &TableNode, ctx: &Ctx, width: f32) -> ColumnWidths {
    let columns = t.header.len().max(1);
    let rows: Vec<&Vec<Vec<Inline>>> = std::iter::once(&t.header).chain(t.rows.iter()).collect();
    let natural: Vec<f32> = (0..columns)
        .map(|c| {
            let lines = |(r, cells): (usize, &&Vec<Vec<Inline>>)| {
                cells
                    .get(c)
                    .map_or(0.0, |inlines| ctx.editor.table_widths.widest(&cell_lines(inlines, cell_base(r, ctx))))
            };
            rows.iter().enumerate().map(lines).fold(0.0, f32::max)
        })
        .collect();
    ctx.column_widths(&natural, width)
}

/// A table that is not being edited, in a page column of `width`.
fn table_view<'a>(t: &TableNode, ctx: &Ctx<'a>, block: usize, width: f32) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let rows: Vec<&Vec<Vec<Inline>>> = std::iter::once(&t.header).chain(t.rows.iter()).collect();
    let base = |r: usize| cell_base(r, ctx);
    let widths = rendered_widths(t, ctx, width);
    let mut grid = Column::new();
    for (r, cells) in rows.iter().enumerate() {
        let mut contents = Row::new();
        let mut frames = Row::new();
        for (c, &w) in widths.widths.iter().enumerate() {
            let inlines = cells.get(c).cloned().unwrap_or_default();
            let header = r == 0;
            let align_x = match t.align.get(c) {
                Some(pulldown_cmark::Alignment::Center) => Alignment::Center,
                Some(pulldown_cmark::Alignment::Right) => Alignment::End,
                _ => Alignment::Start,
            };
            contents = contents.push(
                container(inline_text(&inlines, base(r), ctx, None, block))
                    .width(w)
                    .padding(CELL_PADDING)
                    .align_x(align_x),
            );
            // The whole cell (beside and below its text too) takes the clicks.
            frames = frames.push(
                mouse_area(cell_frame(w, if header { p.header_bg } else { Color::TRANSPARENT }, p))
                    .on_press(EditorMsg::TableCellPressed(block, r, c))
                    .on_right_press(EditorMsg::RightClicked(RightClick::TableCell(block, r, c))),
            );
        }
        grid = grid.push(table_row(contents, frames));
    }
    let start = ctx.editor.page.doc().blocks.get(block).map_or(usize::MAX, |b| b.range.start);
    sideways(grid.into(), widths.scrolls, None, move |x| EditorMsg::TableScrolled(start, x))
}

/// A table's grid, scrolled sideways (its scroll bar below it) when it is
/// wider than the page; `on_scroll` is told how far it is scrolled.
pub(super) fn sideways<'a>(
    grid: Element<'a, EditorMsg>,
    scrolls: bool,
    id: Option<iced::widget::Id>,
    on_scroll: impl Fn(f32) -> EditorMsg + 'a,
) -> Element<'a, EditorMsg> {
    if !scrolls {
        return grid;
    }
    let bar = scrollable::Scrollbar::new().spacing(2);
    let mut s = scrollable(grid)
        .direction(scrollable::Direction::Horizontal(bar))
        .width(Fill)
        .on_scroll(move |v| on_scroll(v.absolute_offset().x));
    if let Some(id) = id {
        s = s.id(id);
    }
    s.into()
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
                let level = (*level).clamp(1, 6);
                let scale = HEADING_SCALE[usize::from(level) - 1];
                let inlines = content::inlines(doc, doc.block_events(i));
                let base =
                    TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: heading::text_color(level, p) };
                heading::framed(level, base.size, width, ctx.zoom, p, |w| inline_text(&inlines, base, ctx, Some(w), i))
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
            BlockKind::Table => table_view(&content::table_block(doc, i), ctx, i, width),
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
