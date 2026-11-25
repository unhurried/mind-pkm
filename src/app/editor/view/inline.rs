//! Rendered text: inline runs laid out in pieces of one line, with their
//! links and find matches.

use super::blocks::{embed_view, image_view, math_view};
use super::hit::{HitInfo, HitRun};
use super::rich::rich;
use super::table_widths::Piece;
use super::theme::{CELL_PADDING, LINE_HEIGHT, bold, code_font, italic};
use super::{Ctx, SpanLink, TextBase};
use crate::app::editor::address;
use crate::app::editor::{EditorMsg, RightClick, TableMsg};
use crate::app::press_point::press_in;
use crate::markdown::content::{Inline, Link, SourceMap, Style};
use iced::advanced::text::Wrapping;
use iced::widget::text::Span;
use iced::widget::{Column, Row, container, mouse_area, pin, space, stack, text};
use iced::{Alignment, Color, Element, Fill, Font};
use std::ops::Range;
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

/// A paragraph: lines of text, or images, math and embeds among the text.
pub(super) fn paragraph_view<'a>(
    inlines: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: f32,
    block: usize,
) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let is_text = |i: &Inline| matches!(i, Inline::Text { .. } | Inline::Break { .. } | Inline::FootnoteRef { .. });
    if inlines.iter().all(is_text) {
        return inline_text(inlines, base, ctx, Some(width), Some(block));
    }
    // Lines (split at breaks) of flowing pieces.
    let mut col = Column::new().spacing(4);
    for line in inlines.split(|i| matches!(i, Inline::Break { .. })) {
        let blank = |i: &Inline| matches!(i, Inline::Text { text, .. } if text.trim().is_empty());
        let only_images = line.iter().all(|i| matches!(i, Inline::Image { .. }) || blank(i));
        if only_images && line.iter().any(|i| matches!(i, Inline::Image { .. })) {
            for i in line {
                if let Inline::Image { .. } = i {
                    col = col.push(image_view(i, ctx, width, block));
                }
            }
            continue;
        }
        let line_end = line.last().map(|i| i.source().end);
        let mut flow = Row::new().align_y(Alignment::Center);
        let mut texts: Vec<Inline> = Vec::new();
        let flush = |texts: &mut Vec<Inline>, flow: Row<'a, EditorMsg>| -> Row<'a, EditorMsg> {
            let mut flow = flow;
            for word in split_words(texts) {
                flow = flow.push(inline_text(&word, base, ctx, None, Some(block)));
            }
            texts.clear();
            flow
        };
        for i in line {
            match i {
                Inline::Text { .. } | Inline::FootnoteRef { .. } => texts.push(i.clone()),
                Inline::Math { tex, display, .. } => {
                    flow = flush(&mut texts, flow);
                    flow = flow.push(math_view(tex, *display, ctx));
                }
                Inline::Image { .. } => {
                    flow = flush(&mut texts, flow);
                    flow = flow.push(image_view(i, ctx, width, block));
                }
                Inline::Embed { tag, src, .. } => {
                    flow = flush(&mut texts, flow);
                    flow = flow.push(embed_view(tag, src.as_deref(), p, ctx));
                }
                Inline::Break { .. } => {}
            }
        }
        flow = flush(&mut texts, flow);
        // A click to the right of the line puts the cursor at its end.
        let line = container(flow.wrap()).width(Fill);
        col = col.push(match line_end {
            Some(end) => mouse_area(line)
                .on_press(EditorMsg::PressedAt(end))
                .on_right_press(EditorMsg::RightClicked(RightClick::At(end)))
                .into(),
            None => Element::from(line),
        });
    }
    col.into()
}

/// Text runs split into words and individual characters in CJK text and URLs,
/// so that they can wrap around inline objects in a row.
fn split_words(runs: &[Inline]) -> Vec<Vec<Inline>> {
    let mut out = Vec::new();
    for (run, breaks) in runs.iter().zip(line_breaks(runs)) {
        let Inline::Text { text, style, link, map } = run else {
            out.push(vec![run.clone()]);
            continue;
        };
        let mut breaks = breaks.into_iter().peekable();
        let mut start = 0;
        let mut chars = text.grapheme_indices(true).peekable();
        while let Some((i, chars_here)) = chars.next() {
            let next = i + chars_here.len();
            let cjk = chars_here.starts_with(|c: char| c as u32 >= 0x2e80);
            let next_cjk = chars.peek().is_some_and(|(_, s)| s.starts_with(|c: char| c as u32 >= 0x2e80));
            let address_break = breaks.next_if_eq(&next).is_some();
            if chars_here.starts_with(char::is_whitespace) || cjk || next_cjk || address_break || next == text.len() {
                out.push(vec![Inline::Text {
                    text: text[start..next].to_string(),
                    style: *style,
                    link: link.clone(),
                    map: map.slice(start..next),
                }]);
                start = next;
            }
        }
    }
    out
}

/// The address break positions in each run. A URL can span multiple styles
/// in the same link label. Inline objects and link boundaries end the scan.
fn line_breaks(runs: &[Inline]) -> Vec<Vec<usize>> {
    let mut line = String::new();
    let mut previous_link = None;
    let ranges: Vec<Range<usize>> = runs
        .iter()
        .map(|run| match run {
            Inline::Text { text, link, .. } => {
                if previous_link != link.as_ref() {
                    line.push(' ');
                }
                previous_link = link.as_ref();
                let start = line.len();
                line.push_str(text);
                start..line.len()
            }
            _ => {
                previous_link = None;
                line.push(' ');
                line.len()..line.len()
            }
        })
        .collect();
    let mut breaks = address::breaks(&line).into_iter().peekable();
    ranges
        .into_iter()
        .map(|range| std::iter::from_fn(|| breaks.next_if(|&at| at <= range.end)).map(|at| at - range.start).collect())
        .collect()
}

/// The spans of text runs (no images), as one text per line. With a page
/// block index, each reports the pointer for mapping clicks to page offsets.
pub(super) fn inline_text<'a>(
    inlines: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: Option<f32>,
    block: Option<usize>,
) -> Element<'a, EditorMsg> {
    text_lines(inlines, base, ctx, width, block, None)
}

/// The lines of a table cell, aligned individually within its padding.
pub(super) fn cell_text<'a>(
    inlines: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    cell_width: f32,
    block: Option<usize>,
    align_x: Alignment,
) -> Element<'a, EditorMsg> {
    let width = (cell_width - 2.0 * CELL_PADDING[1]).max(0.0);
    text_lines(inlines, base, ctx, Some(width), block, Some(align_x))
}

fn text_lines<'a>(
    inlines: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: Option<f32>,
    block: Option<usize>,
    cell_align: Option<Alignment>,
) -> Element<'a, EditorMsg> {
    let mut lines: Vec<Element<'a, EditorMsg>> = Vec::new();
    for line in inlines.split(|i| matches!(i, Inline::Break { .. })) {
        lines.push(text_line(line, base, ctx, width, block, cell_align));
    }
    if lines.len() == 1 {
        return lines.pop().unwrap_or_else(|| space().into());
    }
    let column = Column::with_children(lines);
    match cell_align {
        Some(align) => column.width(Fill).align_x(align).into(),
        None => column.into(),
    }
}

/// One line of text runs.
fn text_line<'a>(
    runs: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: Option<f32>,
    block: Option<usize>,
    cell_align: Option<Alignment>,
) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let mut spans: Vec<Span<'static, SpanLink, Font>> = Vec::new();
    let mut hits: Vec<HitRun> = Vec::new();
    for (run, breaks) in runs.iter().zip(line_breaks(runs)) {
        match run {
            Inline::Text { text: t, style, link, map } => {
                let (size, font) = run_font(*style, base);
                let mut color = if link.is_some() { p.link } else { base.color };
                if let Some(c) = style.color {
                    color = Color::from_rgb8(c.0, c.1, c.2);
                }
                if style.dim {
                    color = p.muted;
                }
                if style.code {
                    color = p.code;
                }
                // Pieces in and out of find matches.
                for (piece, found) in match_pieces(t, map, ctx) {
                    let mut span =
                        Span::new(address::layout_text(t, piece, &breaks)).size(size).font(font).color(color);
                    if style.underline || link.is_some() {
                        span = span.underline(true);
                    }
                    if style.strike {
                        span = span.strikethrough(true);
                    }
                    if let Some(current) = found {
                        span = span.background(if current { p.find } else { p.find_bg });
                    } else if style.highlight {
                        span = span.background(p.mark_bg);
                    } else if style.code || style.kbd {
                        span = span.background(p.code_bg);
                    }
                    if let Some(Link { dest, .. }) = link {
                        span = span.link(SpanLink { dest: dest.clone(), at: map.source_range().start });
                    }
                    spans.push(span);
                }
                hits.push(HitRun { text: t.clone(), breaks, size, font, map: Some(map.clone()) });
            }
            Inline::FootnoteRef { label, number, source } => {
                let size = base.size * 0.75;
                spans.push(
                    Span::new(format!("[{number}]"))
                        .size(size)
                        .color(p.link)
                        .link(SpanLink { dest: format!("^{label}"), at: source.start }),
                );
                hits.push(HitRun { text: format!("[{number}]"), breaks: Vec::new(), size, font: base.font, map: None });
            }
            _ => {}
        }
    }
    if spans.is_empty() {
        // An empty line keeps its height.
        return text(" ").size(base.size).line_height(LINE_HEIGHT).into();
    }
    let align_x = cell_align.map_or_else(Default::default, Into::into);
    let mut rich = rich(spans, &ctx.editor.decoration_grid)
        .size(base.size)
        .font(base.font)
        .line_height(LINE_HEIGHT)
        .align_x(align_x)
        .wrapping(Wrapping::WordOrGlyph);
    // A cell aligns each paragraph within its frame, so keep its width shrinkable.
    if cell_align.is_none()
        && let Some(w) = width
    {
        rich = rich.width(w);
    }
    let Some(block) = block else {
        // The offsets in this text are local to the cell. Its frame takes
        // ordinary presses, and following a link needs only the destination.
        if ctx.editor.modifiers.control() || ctx.editor.modifiers.command() {
            rich = rich.on_link_click(|l: SpanLink| EditorMsg::Table(TableMsg::Link(l.dest)));
        }
        return rich.into();
    };
    let rich = rich.on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at));
    let info = Arc::new(HitInfo { block, runs: hits, width: width.unwrap_or(f32::INFINITY), align_x });
    // Where an item dragged from the assets pane would go, in this text.
    let mark = ctx.editor.drop_mark_in(&info);
    let mut area =
        mouse_area(rich).on_press(EditorMsg::TextPressed).on_right_press(EditorMsg::RightClicked(RightClick::Text));
    // While an item of the assets pane is dragged, where the pointer is in the text.
    if ctx.editor.asset_dragging() {
        let info = info.clone();
        area = area.on_move(move |pt| EditorMsg::PointerOver(info.clone(), pt));
    }
    // Where a press lands in the text, before the press itself.
    let area = press_in(area, move |pt| EditorMsg::PointerOver(info.clone(), pt));
    match mark {
        Some(r) => stack![
            area,
            pin(container(space()).width(r.width).height(r.height).style(move |_| container::background(p.link)))
                .position(r.position())
        ]
        .into(),
        None => area.into(),
    }
}

/// A table cell's lines (split at its breaks) as pieces of text to measure,
/// each set as `text_line` sets it.
pub(super) fn cell_lines(inlines: &[Inline], base: TextBase) -> Vec<Vec<Piece>> {
    inlines
        .split(|i| matches!(i, Inline::Break { .. }))
        .map(|line| {
            line.iter()
                .filter_map(|run| match run {
                    Inline::Text { text, style, .. } => {
                        let (size, font) = run_font(*style, base);
                        Some((text.clone(), size, font))
                    }
                    Inline::FootnoteRef { number, .. } => Some((format!("[{number}]"), base.size * 0.75, base.font)),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// A text run cut where find matches start and end: each piece, and whether
/// it is in a match (and in the selected one).
fn match_pieces(t: &str, map: &SourceMap, ctx: &Ctx) -> Vec<(std::ops::Range<usize>, Option<bool>)> {
    let source = map.source_range();
    if !ctx.matches.iter().any(|m| m.start < source.end && m.end > source.start) {
        return vec![(0..t.len(), None)];
    }
    let state = |i: usize| {
        let s = map.source_of(i);
        let k = ctx.matches.partition_point(|m| m.end <= s);
        ctx.matches.get(k).filter(|m| m.start <= s).map(|m| ctx.current == Some((m.start, m.end)))
    };
    let mut pieces: Vec<(std::ops::Range<usize>, Option<bool>)> = Vec::new();
    for (i, _) in t.char_indices() {
        let found = state(i);
        match pieces.last_mut() {
            Some((r, f)) if *f == found => r.end = i,
            _ => pieces.push((i..i, found)),
        }
    }
    for k in 0..pieces.len() {
        pieces[k].0.end = pieces.get(k + 1).map_or(t.len(), |p| p.0.start);
    }
    pieces
}

/// The size and font of a run's style.
pub(super) fn run_font(style: Style, base: TextBase) -> (f32, Font) {
    let mut font = if style.code || style.kbd { code_font() } else { base.font };
    if style.bold {
        font = bold(font);
    }
    if style.italic {
        font = italic(font);
    }
    let size = if style.sub || style.sup { base.size * 0.75 } else { base.size };
    (size, font)
}
