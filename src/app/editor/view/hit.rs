//! Mapping the pointer over a piece of rendered text back to the source.

use super::theme::{BODY_SIZE, LINE_HEIGHT};
use crate::app::view::APP_FONT;
use crate::markdown::content::SourceMap;
use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};
use iced::widget::text::{LineHeight, Span};
use iced::{Font, Point, Size};

/// A piece of rendered text (one line, no breaks) and where its characters
/// come from, for mapping a click to the source.
#[derive(Debug, Clone)]
pub struct HitInfo {
    pub block: usize,
    pub runs: Vec<HitRun>,
    /// The width the text was laid out in.
    pub width: f32,
}

#[derive(Debug, Clone)]
pub struct HitRun {
    pub text: String,
    pub size: f32,
    pub font: Font,
    /// None for text that is no source (a bullet, an emoji's image...).
    pub map: Option<SourceMap>,
}

/// A piece of text laid out as the rendered text widget lays it out.
fn hit_paragraph(info: &HitInfo) -> iced::advanced::graphics::text::Paragraph {
    let spans: Vec<Span<'_, (), Font>> =
        info.runs.iter().map(|r| Span::new(r.text.as_str()).size(r.size).font(r.font)).collect();
    let size = info.runs.first().map_or(BODY_SIZE, |r| r.size);
    iced::advanced::graphics::text::Paragraph::with_spans(iced::advanced::Text {
        content: spans.as_slice(),
        bounds: Size::new(info.width, f32::INFINITY),
        size: size.into(),
        line_height: LineHeight::Relative(LINE_HEIGHT),
        font: APP_FONT,
        align_x: iced::advanced::text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::WordOrGlyph,
    })
}

/// Which piece of rendered text this is: its block and where its source starts.
pub fn piece_key(info: &HitInfo) -> (usize, usize) {
    let start = info.runs.iter().find_map(|r| r.map.as_ref().map(|m| m.source_range().start));
    (info.block, start.unwrap_or(usize::MAX))
}

/// The caret between the characters nearest a point in a piece of text
/// (where something dropped there goes).
pub fn caret_in(info: &HitInfo, p: Point) -> Option<iced::Rectangle> {
    let paragraph = hit_paragraph(info);
    let buffer = paragraph.buffer();
    let cursor = buffer.hit(p.x, p.y)?;
    let run = buffer
        .layout_runs()
        .filter(|r| r.line_i == cursor.line)
        .find(|r| p.y < r.line_top + r.line_height)
        .or_else(|| buffer.layout_runs().filter(|r| r.line_i == cursor.line).last())?;
    let x = run
        .glyphs
        .iter()
        .find(|g| g.start >= cursor.index)
        .map(|g| g.x)
        .or_else(|| run.glyphs.last().map(|g| g.x + g.w))
        .unwrap_or(0.0);
    Some(iced::Rectangle::new(Point::new(x - 1.0, run.line_top), Size::new(2.0, run.line_height)))
}

/// The source offset a point in a piece of text stands for.
pub fn hit_test(info: &HitInfo, p: Point) -> Option<usize> {
    let paragraph = hit_paragraph(info);
    let iced::advanced::text::Hit::CharOffset(index) = paragraph.hit_test(p)?;
    let mut start = 0;
    let mut last = None;
    for run in &info.runs {
        let end = start + run.text.len();
        if let Some(map) = &run.map {
            if index < end || (index == end && run.map.is_some()) {
                return Some(map.source_of(index.saturating_sub(start).min(run.text.len())));
            }
            last = Some(map.source_range().end);
        }
        start = end;
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::content;

    fn run(text: &str, source: std::ops::Range<usize>) -> HitRun {
        HitRun {
            text: text.into(),
            size: BODY_SIZE,
            font: APP_FONT,
            map: Some(content::source_map(text.len(), source)),
        }
    }

    #[test]
    fn a_click_maps_to_the_source_of_the_text_under_it() {
        // "hello world" rendered from `hello **world**`: "world" comes from 8..13.
        let info = HitInfo { block: 0, runs: vec![run("hello ", 0..6), run("world", 8..13)], width: 1000.0 };
        assert_eq!(hit_test(&info, Point::new(0.0, 5.0)), Some(0));
        assert_eq!(hit_test(&info, Point::new(999.0, 5.0)), Some(13));
        // Somewhere inside "world".
        let inside = hit_test(&info, Point::new(70.0, 5.0)).unwrap();
        assert!((8..=13).contains(&inside), "{inside}");
    }

    #[test]
    fn wrapped_lines_are_hit_by_their_height() {
        let text = "word ".repeat(30);
        let info = HitInfo { block: 0, runs: vec![run(&text, 100..100 + text.len())], width: 120.0 };
        let first = hit_test(&info, Point::new(1.0, 5.0)).unwrap();
        let third = hit_test(&info, Point::new(1.0, 2.5 * BODY_SIZE * LINE_HEIGHT)).unwrap();
        assert_eq!(first, 100);
        assert!(third > 110, "{third}");
    }
}
