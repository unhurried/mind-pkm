//! The widths of a table's columns: each as wide as its widest line of text,
//! up to a maximum (wider text wraps at the maximum); a table narrower than
//! the page widened to it, a wider one scrolled sideways.
//!
//! The rendered table and the table being edited take their widths from here
//! alike, so that entering a table moves none of its columns.

use super::theme::{CELL_PADDING, LINE_HEIGHT, TABLE_COLUMN_MAX};
use crate::app::view::APP_FONT;
use iced::Font;
use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};
use iced::widget::text::{LineHeight, Span};
use std::cell::RefCell;
use std::collections::HashMap;

/// Added to the width of a column whose text fits, so that rounding never
/// wraps a line measured to fit.
const FIT_ALLOWANCE: f32 = 1.0;

/// How many measured lines the cache keeps before it starts again.
const CACHE_LIMIT: usize = 4096;

/// A piece of a line of text: the text, its size and font.
pub(super) type Piece = (String, f32, Font);

/// The columns' widths (of the cells, their padding included) and whether
/// they make the table wider than the page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ColumnWidths {
    pub widths: Vec<f32>,
    pub scrolls: bool,
}

/// The widths of columns whose widest lines are `natural` wide: a column
/// takes its text's width if that is at most `max`, else `max`, plus the
/// cells' `padding` (left and right together); then the columns are widened
/// in proportion to fill `page` if they are narrower, or kept and scrolled
/// if they are wider.
pub(super) fn column_widths(natural: &[f32], max: f32, padding: f32, page: f32) -> ColumnWidths {
    let mut widths: Vec<f32> =
        natural.iter().map(|&n| if n <= max { n + FIT_ALLOWANCE } else { max } + padding).collect();
    let sum: f32 = widths.iter().sum();
    if sum > page {
        return ColumnWidths { widths, scrolls: true };
    }
    if sum > 0.0 {
        let k = page / sum;
        for w in &mut widths {
            *w *= k;
        }
    }
    ColumnWidths { widths, scrolls: false }
}

impl super::Ctx<'_> {
    /// The widths of columns whose widest lines are `natural` wide, in a
    /// page column of `page`.
    pub(super) fn column_widths(&self, natural: &[f32], page: f32) -> ColumnWidths {
        column_widths(natural, TABLE_COLUMN_MAX * self.zoom, 2.0 * CELL_PADDING[1], page)
    }
}

/// The widths of lines of text as measured, kept between views.
#[derive(Debug, Default)]
pub struct WidthCache(RefCell<HashMap<LineKey, f32>>);

/// A measured line: its pieces' texts, sizes (as bits) and fonts.
type LineKey = Vec<(String, u32, Font)>;

impl WidthCache {
    /// The width of the widest of `lines` laid out on one line each.
    pub(super) fn widest(&self, lines: &[Vec<Piece>]) -> f32 {
        lines.iter().map(|l| self.width(l)).fold(0.0, f32::max)
    }

    fn width(&self, line: &[Piece]) -> f32 {
        if line.iter().all(|(t, _, _)| t.is_empty()) {
            return 0.0;
        }
        let key: LineKey = line.iter().map(|(t, s, f)| (t.clone(), s.to_bits(), *f)).collect();
        let mut cache = self.0.borrow_mut();
        if let Some(&w) = cache.get(&key) {
            return w;
        }
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        let w = line_width(line);
        cache.insert(key, w);
        w
    }
}

/// A line of text laid out as the rendered text widget lays it out, in `width`.
fn paragraph(line: &[Piece], width: f32) -> iced::advanced::graphics::text::Paragraph {
    let spans: Vec<Span<'_, (), Font>> = line.iter().map(|(t, s, f)| Span::new(t.as_str()).size(*s).font(*f)).collect();
    let size = line.first().map_or(16.0, |p| p.1);
    iced::advanced::graphics::text::Paragraph::with_spans(iced::advanced::Text {
        content: spans.as_slice(),
        bounds: iced::Size::new(width, f32::INFINITY),
        size: size.into(),
        line_height: LineHeight::Relative(LINE_HEIGHT),
        font: APP_FONT,
        align_x: iced::advanced::text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::WordOrGlyph,
    })
}

/// The width of a line of text on one line.
fn line_width(line: &[Piece]) -> f32 {
    paragraph(line, f32::INFINITY).min_bounds().width
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAD: f32 = 16.0;

    #[test]
    fn fitting_columns_take_their_text_s_width_and_fill_the_page() {
        let w = column_widths(&[20.0, 40.0], 300.0, PAD, 800.0);
        assert!(!w.scrolls);
        assert!((w.widths.iter().sum::<f32>() - 800.0).abs() < 0.01, "{w:?}");
        // In proportion to their widths: (20 + 1 + 16) to (40 + 1 + 16).
        assert!((w.widths[1] / w.widths[0] - 57.0 / 37.0).abs() < 0.001, "{w:?}");
    }

    #[test]
    fn a_column_that_would_wrap_takes_the_maximum() {
        let w = column_widths(&[10.0, 1000.0], 300.0, PAD, 2000.0);
        assert!((w.widths[1] / w.widths[0] - 316.0 / 27.0).abs() < 0.001, "{w:?}");
        // The page is narrower than the columns at the maximum: no widening.
        let w = column_widths(&[10.0, 1000.0], 300.0, PAD, 343.0);
        assert_eq!(w, ColumnWidths { widths: vec![27.0, 316.0], scrolls: false });
    }

    #[test]
    fn columns_wider_than_the_page_scroll() {
        let w = column_widths(&[200.0; 6], 300.0, PAD, 800.0);
        assert!(w.scrolls);
        assert_eq!(w.widths, vec![217.0; 6]);
    }

    #[test]
    fn no_columns() {
        assert_eq!(column_widths(&[], 300.0, PAD, 800.0), ColumnWidths { widths: vec![], scrolls: false });
    }

    #[test]
    fn a_line_laid_out_in_its_measured_width_stays_on_one_line() {
        let cache = WidthCache::default();
        let lines = [
            vec![("Next Steps for the week".to_string(), 15.0, APP_FONT)],
            vec![("今週の予定を".to_string(), 15.0, APP_FONT), ("確認する".to_string(), 15.0, bold())],
        ];
        for line in &lines {
            let w = cache.widest(std::slice::from_ref(line));
            assert!(w > 0.0);
            let one = paragraph(line, f32::INFINITY).min_bounds().height;
            let fitted = paragraph(line, w + FIT_ALLOWANCE).min_bounds().height;
            assert_eq!(fitted, one, "{line:?} wraps in {w}");
            // A little narrower, it wraps.
            assert!(paragraph(line, w - 8.0).min_bounds().height > one, "{line:?}");
        }
        assert_eq!(cache.widest(&[vec![], vec![(String::new(), 15.0, APP_FONT)]]), 0.0);
    }

    fn bold() -> Font {
        Font { weight: iced::font::Weight::Bold, ..APP_FONT }
    }

    /// The editor of a page, with a context to render it at 100%.
    fn page(text: &str) -> crate::app::editor::Editor {
        let page = crate::app::editor::page::Page::new("/tmp/page.md".into(), text.to_string(), false);
        crate::app::editor::Editor::new(page)
    }

    fn with_ctx<T>(e: &crate::app::editor::Editor, f: impl FnOnce(&super::super::Ctx) -> T) -> T {
        let renders = crate::app::editor::RenderCache::default();
        let ctx = super::super::Ctx {
            editor: e,
            p: super::super::Palette::new(false),
            zoom: 1.0,
            renders: &renders,
            matches: &[],
            current: None,
        };
        f(&ctx)
    }

    fn rendered(e: &crate::app::editor::Editor, width: f32) -> ColumnWidths {
        let t = crate::markdown::content::table_block(e.page.doc(), 0);
        with_ctx(e, |ctx| super::super::blocks::rendered_widths(&t, ctx, width))
    }

    fn edited(e: &crate::app::editor::Editor, width: f32) -> ColumnWidths {
        let a = e.page.active().expect("the table is edited");
        let state = a.table().expect("a table");
        with_ctx(e, |ctx| e.edited_widths(state, &e.edited_cells(a, state, ctx), ctx, width))
    }

    const LONG: &str = "このタスクは来週までに資料をまとめてチームに共有し、レビューの結果を反映する予定です";

    #[test]
    fn short_and_long_columns_of_a_rendered_table() {
        let e = page(&format!("| No | 説明 |\n| --- | --- |\n| 1 | {LONG} |\n"));
        let w = rendered(&e, 820.0);
        assert!(!w.scrolls);
        assert!((w.widths.iter().sum::<f32>() - 820.0).abs() < 0.01, "{w:?}");
        assert!(w.widths[1] > 5.0 * w.widths[0], "{w:?}");
        // The long column at the maximum, widened like the other to the page.
        let text = w.widths[1] - 16.0;
        assert!(text > TABLE_COLUMN_MAX, "{w:?}");
    }

    #[test]
    fn a_rendered_table_wider_than_the_page_scrolls() {
        let row = |s: &str| format!("|{}\n", format!(" {s} |").repeat(8));
        let e = page(&format!("{}{}{}", row("a"), row("---"), row("ある程度の長さのセル")));
        let w = rendered(&e, 820.0);
        assert!(w.scrolls, "{w:?}");
        assert!(w.widths.iter().sum::<f32>() > 820.0);
    }

    #[test]
    fn the_edited_table_has_the_rendered_widths_and_follows_typing() {
        use crate::app::editor::{EditorMsg, EditorPrefs, LiveMsg};
        let mut e = page(&format!("| No | 説明 |\n| --- | --- |\n| 1 | {LONG} |\n"));
        let before = rendered(&e, 820.0);
        e.page.place(crate::markdown::edit::Sel::caret(0));
        e.page.go_to_cell_at((1, 0), true);
        assert_eq!(edited(&e, 820.0), before);
        // Typing into the number column widens it.
        let prefs = EditorPrefs::default();
        let _ = e.update(EditorMsg::Live(LiveMsg::Insert("2345678901234567890".into())), &prefs, &mut Vec::new());
        assert!(e.page.text().contains("| 12345678901234567890 |"), "{}", e.page.text());
        let typed = edited(&e, 820.0);
        assert!(typed.widths[0] > 2.0 * before.widths[0], "{before:?} {typed:?}");
        assert_eq!(rendered(&e, 820.0), typed);
    }
}
