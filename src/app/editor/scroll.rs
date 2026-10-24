//! Scrolling the page: keeping the cursor in view as it moves or the text
//! grows, and bringing a block (a heading, a match) to the top.
//!
//! The text editor of the active region does not scroll the page around it,
//! so the editor asks the widget tree where the region's box and the page's
//! scrollable are, works out where the cursor's line is in the box, and
//! scrolls when that line is outside the visible part.

use super::EditorMsg;
use crate::app::view::APP_FONT;
use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};
use iced::advanced::widget::operation::{Operation, Outcome, Scrollable};
use iced::advanced::widget::{self, Id};
use iced::widget::text::LineHeight;
use iced::widget::{operation, scrollable};
use iced::{Font, Rectangle, Size, Task, Vector};

/// Room kept between the cursor and the edges of the visible part.
const MARGIN: f32 = 40.0;

/// Where a container and a scrollable are.
struct Find {
    target: Id,
    scroll: Id,
    target_bounds: Option<Rectangle>,
    /// The scrollable's bounds and how far it is scrolled.
    scroll_state: Option<(Rectangle, Vector)>,
}

#[derive(Debug, Clone, Copy)]
struct Found {
    target: Rectangle,
    viewport: Rectangle,
    offset: Vector,
}

impl Operation<Option<Found>> for Find {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Found>>)) {
        operate(self)
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            self.target_bounds = Some(bounds);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&self.scroll) {
            self.scroll_state = Some((bounds, translation));
        }
    }

    fn finish(&self) -> Outcome<Option<Found>> {
        Outcome::Some(match (self.target_bounds, self.scroll_state) {
            (Some(target), Some((viewport, offset))) => Some(Found { target, viewport, offset }),
            _ => None,
        })
    }
}

fn find(target: Id, scroll: Id) -> Task<Option<Found>> {
    widget::operate(Find { target, scroll, target_bounds: None, scroll_state: None })
}

/// How the active region's text is set, to find the cursor's line in it.
#[derive(Debug, Clone)]
pub struct CaretText {
    /// The region's text up to the cursor.
    pub before: String,
    pub size: f32,
    pub font: Font,
    pub line_height: f32,
    pub padding: f32,
}

/// The top and bottom of the cursor's line in a box of `width`.
fn caret_span(caret: &CaretText, width: f32) -> (f32, f32) {
    let line = caret.size * caret.line_height;
    // The text up to the cursor, with something after a trailing line break
    // so that its height counts the cursor's (empty) line.
    let text = format!("{} ", caret.before);
    let p = iced::advanced::graphics::text::Paragraph::with_text(iced::advanced::Text {
        content: text.as_str(),
        bounds: Size::new((width - 2.0 * caret.padding).max(1.0), f32::INFINITY),
        size: caret.size.into(),
        line_height: LineHeight::Relative(caret.line_height),
        font: if caret.font == Font::DEFAULT { APP_FONT } else { caret.font },
        align_x: iced::advanced::text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::WordOrGlyph,
    });
    let bottom = caret.padding + p.min_bounds().height.max(line);
    (bottom - line, bottom)
}

/// The scroll offset that brings `top..bottom` (in the page's coordinates)
/// into the visible part, if it isn't.
fn offset_to_show(top: f32, bottom: f32, viewport: Rectangle, offset: Vector) -> Option<f32> {
    let visible_top = viewport.y + offset.y;
    let visible_bottom = visible_top + viewport.height;
    if top < visible_top + MARGIN / 2.0 {
        Some((top - viewport.y - MARGIN).max(0.0))
    } else if bottom > visible_bottom - MARGIN / 2.0 {
        Some(bottom - viewport.y - viewport.height + MARGIN)
    } else {
        None
    }
}

/// Scrolls the page so that the cursor in the active region's box is visible.
pub fn reveal_caret(region_box: Id, page: Id, caret: CaretText) -> Task<EditorMsg> {
    let page_id = page.clone();
    find(region_box, page).then(move |found| {
        let Some(f) = found else { return Task::none() };
        let (top, bottom) = caret_span(&caret, f.target.width);
        match offset_to_show(f.target.y + top, f.target.y + bottom, f.viewport, f.offset) {
            Some(y) => operation::scroll_to(page_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y }),
            None => Task::none(),
        }
    })
}

/// Scrolls the page so that the box `target` is at the top.
pub fn to_top(target: Id, page: Id) -> Task<EditorMsg> {
    let page_id = page.clone();
    find(target, page).then(move |found| {
        let Some(f) = found else { return Task::none() };
        let y = (f.target.y - f.viewport.y - MARGIN / 2.0).max(0.0);
        operation::scroll_to(page_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_only_when_outside() {
        let viewport = Rectangle::new(iced::Point::new(0.0, 30.0), Size::new(800.0, 500.0));
        // Visible: 100..600 of the page (offset 100 from the viewport's top at 30).
        let offset = Vector::new(0.0, 100.0);
        assert_eq!(offset_to_show(300.0, 320.0, viewport, offset), None);
        assert_eq!(offset_to_show(110.0, 130.0, viewport, offset), Some(40.0));
        assert_eq!(offset_to_show(650.0, 670.0, viewport, offset), Some(180.0));
    }

    #[test]
    fn the_cursor_line_is_found_in_wrapped_text() {
        let caret = |before: &str| CaretText {
            before: before.into(),
            size: 15.0,
            font: Font::DEFAULT,
            line_height: 1.6,
            padding: 0.0,
        };
        let (top, bottom) = caret_span(&caret("one"), 400.0);
        assert_eq!((top, bottom), (0.0, 24.0));
        let (top, _) = caret_span(&caret("one\ntwo\n"), 400.0);
        assert_eq!(top, 48.0);
        // A long line wraps in a narrow box.
        let (top, _) = caret_span(&caret(&"word ".repeat(40)), 100.0);
        assert!(top > 100.0, "{top}");
    }
}
