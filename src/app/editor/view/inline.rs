//! Rendered text: inline runs laid out in pieces of one line, with their
//! links and find matches.

use super::blocks::{embed_view, image_view, math_view};
use super::hit::{HitInfo, HitRun};
use super::theme::{CODE_FONT, LINE_HEIGHT, bold, italic};
use super::{Ctx, SpanLink, TextBase};
use crate::app::editor::EditorMsg;
use crate::markdown::content::{Inline, Link, SourceMap, Style};
use iced::advanced::text::Wrapping;
use iced::widget::text::Span;
use iced::widget::{Column, Row, container, mouse_area, pin, rich_text, space, stack, text};
use iced::{Alignment, Color, Element, Fill, Font};
use std::sync::Arc;

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
        return inline_text(inlines, base, ctx, Some(width), block);
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
                flow = flow.push(inline_text(&word, base, ctx, None, block));
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
            Some(end) => mouse_area(line).on_press(EditorMsg::PressedAt(end)).into(),
            None => Element::from(line),
        });
    }
    col.into()
}

/// Text runs split into words (and single CJK characters), so that they can wrap in a row.
fn split_words(runs: &[Inline]) -> Vec<Vec<Inline>> {
    let mut out = Vec::new();
    for run in runs {
        let Inline::Text { text, style, link, map } = run else {
            out.push(vec![run.clone()]);
            continue;
        };
        let mut start = 0;
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        for (n, (i, c)) in chars.iter().enumerate() {
            let next = chars.get(n + 1).map(|(j, _)| *j).unwrap_or(text.len());
            let cjk = *c as u32 >= 0x2e80;
            let boundary = c.is_whitespace() || cjk || chars.get(n + 1).is_some_and(|(_, d)| *d as u32 >= 0x2e80);
            if boundary || next == text.len() {
                let piece = &text[start..next];
                if !piece.is_empty() {
                    out.push(vec![Inline::Text {
                        text: piece.to_string(),
                        style: *style,
                        link: link.clone(),
                        map: map_slice(map, start..next),
                    }]);
                }
                start = next;
            }
            let _ = i;
        }
    }
    out
}

fn map_slice(map: &SourceMap, range: std::ops::Range<usize>) -> SourceMap {
    map.slice(range)
}

/// The spans of text runs (no images), as one text per line; each reports
/// the pointer for mapping clicks.
pub(super) fn inline_text<'a>(
    inlines: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: Option<f32>,
    block: usize,
) -> Element<'a, EditorMsg> {
    let mut lines: Vec<Element<'a, EditorMsg>> = Vec::new();
    for line in inlines.split(|i| matches!(i, Inline::Break { .. })) {
        lines.push(text_line(line, base, ctx, width, block));
    }
    if lines.len() == 1 {
        return lines.pop().unwrap_or_else(|| space().into());
    }
    Column::with_children(lines).into()
}

/// One line of text runs.
fn text_line<'a>(
    runs: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: Option<f32>,
    block: usize,
) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let mut spans: Vec<Span<'static, SpanLink, Font>> = Vec::new();
    let mut hits: Vec<HitRun> = Vec::new();
    for run in runs {
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
                    let mut span = Span::new(t[piece].to_string()).size(size).font(font).color(color);
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
                hits.push(HitRun { text: t.clone(), size, font, map: Some(map.clone()) });
            }
            Inline::FootnoteRef { label, number, source } => {
                let size = base.size * 0.75;
                spans.push(
                    Span::new(format!("[{number}]"))
                        .size(size)
                        .color(p.link)
                        .link(SpanLink { dest: format!("^{label}"), at: source.start }),
                );
                hits.push(HitRun { text: format!("[{number}]"), size, font: base.font, map: None });
            }
            _ => {}
        }
    }
    if spans.is_empty() {
        // An empty line keeps its height.
        return text(" ").size(base.size).line_height(LINE_HEIGHT).into();
    }
    let mut rich = rich_text(spans)
        .size(base.size)
        .font(base.font)
        .line_height(LINE_HEIGHT)
        .wrapping(Wrapping::WordOrGlyph)
        .on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at));
    if let Some(w) = width {
        rich = rich.width(w);
    }
    let info = Arc::new(HitInfo { block, runs: hits, width: width.unwrap_or(f32::INFINITY) });
    // Where an item dragged from the assets pane would go, in this text.
    let mark = ctx.editor.drop_mark_in(&info);
    let area =
        mouse_area(rich).on_move(move |pt| EditorMsg::PointerOver(info.clone(), pt)).on_press(EditorMsg::TextPressed);
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

/// The size and font of a run's style.
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

pub(super) fn run_font(style: Style, base: TextBase) -> (f32, Font) {
    let mut font = if style.code || style.kbd { CODE_FONT } else { base.font };
    if style.bold {
        font = bold(font);
    }
    if style.italic {
        font = italic(font);
    }
    let size = if style.sub || style.sup { base.size * 0.75 } else { base.size };
    (size, font)
}
