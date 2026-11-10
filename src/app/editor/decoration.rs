//! Place text decorations on the same physical pixel grid as iced's glyphs.

use std::cell::Cell;

pub(super) struct Grid {
    pub scale: Cell<f32>,
    pub scroll: Cell<f32>,
}

impl Default for Grid {
    fn default() -> Self {
        Self { scale: Cell::new(1.0), scroll: Cell::new(0.0) }
    }
}

impl Grid {
    /// `origin` is the paragraph's drawing origin, `baseline` its shaped
    /// baseline relative to that origin, and `top` the desired decoration
    /// top in layout coordinates. Scroll is applied by the parent renderer.
    pub fn top(&self, origin: f32, baseline: f32, top: f32) -> f32 {
        let scale = self.scale.get();
        let scroll = self.scroll.get();
        // cosmic-text hints the glyph's Y origin by truncating it; both
        // iced renderers round the shaped baseline separately. Rounding
        // origin + decoration instead changes the gap on fractional rows.
        let physical_baseline = ((origin - scroll) * scale).trunc() + (baseline * scale).round();
        let below_baseline = ((top - origin - baseline) * scale).round();
        (physical_baseline + below_baseline) / scale + scroll
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::editor::flow;
    use crate::app::editor::view::{BODY_SIZE, TEXT_LINE_HEIGHT as LINE_HEIGHT, code_font};
    use crate::app::view::APP_FONT;
    use iced::advanced::graphics::text::Paragraph;
    use iced::advanced::text::{self, Paragraph as _};
    use iced::widget::text::{Shaping, Wrapping};
    use iced::{Pixels, Size};

    #[test]
    fn decorations_follow_hinted_glyphs_at_fractional_positions() {
        let grid = Grid::default();
        for zoom in [0.7, 0.8, 0.9, 1.0, 1.25] {
            let size = BODY_SIZE * zoom;
            for (content, font) in
                [("あああああああああ", APP_FONT), ("a link a link a link", APP_FONT), ("code code code", code_font())]
            {
                let paragraph = Paragraph::with_text(text::Text {
                    content,
                    // Enough text to wrap, with the same fractional line
                    // heights that repeated list items have when zoomed out.
                    bounds: Size::new(size * 4.0, f32::INFINITY),
                    size: Pixels(size),
                    line_height: LINE_HEIGHT.into(),
                    font,
                    align_x: text::Alignment::Default,
                    align_y: iced::alignment::Vertical::Top,
                    shaping: Shaping::Advanced,
                    wrapping: Wrapping::WordOrGlyph,
                });
                let lines: Vec<_> = paragraph.buffer().layout_runs().collect();
                assert!(lines.len() > 1);
                for scale in [1.0, 1.25, 1.5, 2.0] {
                    grid.scale.set(scale);
                    for decoration in [0, 1] {
                        let mut expected = None;
                        for scroll in [0.0, 0.3, 17.75, 123.45] {
                            grid.scroll.set(scroll);
                            for row in 0..40 {
                                for line in &lines {
                                    let origin = 48.0 + row as f32 * (size * LINE_HEIGHT + 2.0 * zoom);
                                    let tops = flow::decoration_tops(line.line_top, size, LINE_HEIGHT);
                                    let top = if decoration == 0 { tops.0 } else { tops.1 };
                                    let glyph = &line.glyphs[0];
                                    // Use cosmic-text's actual hinting, as both iced
                                    // backends do, rather than an unrounded line box.
                                    let glyph_y = glyph.physical((0.0, (origin - scroll) * scale), scale).y;
                                    let baseline = glyph_y as f32 + (line.line_y * scale).round();
                                    let y = (grid.top(origin, line.line_y, origin + top) - scroll) * scale;
                                    assert!((y - y.round()).abs() < 0.001, "decoration is not on a physical pixel");
                                    let gap = y - baseline;
                                    let first = *expected.get_or_insert(gap);
                                    assert!(
                                        (gap - first).abs() < 0.001,
                                        "{content}, zoom {zoom}, scale {scale}, scroll {scroll}, row {row}: gap {gap}, expected {first}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
