//! Scrolling the page: keeping the cursor in view as it moves or the text
//! grows, and bringing a block (a heading, a match) to the top.
//!
//! The text editor of the active region does not scroll the page around it,
//! so the editor asks the widget tree where the region's box and the page's
//! scrollable are, works out where the cursor's line is in the box, and
//! scrolls when that line is outside the visible part.
//!
//! Finding and scrolling are one widget operation: finding chains the scroll
//! (`Outcome::Chain`), which iced runs in the same pass, right after the
//! update and before the window is redrawn. A scroll issued by a task that
//! waits for what was found would come back later from another thread, and
//! iced does not redraw for it - the page would only show it at the next
//! redraw (when the pointer leaves the clicked outline entry, say).

use super::EditorMsg;
use crate::app::view::APP_FONT;
use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};
use iced::advanced::widget::operation::{self, Operation, Outcome, Scrollable};
use iced::advanced::widget::{self, Id};
use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::text::LineHeight;
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

fn find(target: Id, scroll: Id) -> Find {
    Find { target, scroll, target_bounds: None, scroll_state: None }
}

/// Where to scroll the page to (its offset), given what was found; `None`
/// leaves it where it is.
type Decide = Box<dyn Fn(Found) -> Option<f32> + Send>;

/// Finds with `find`, then scrolls the page `scroll` to where `decide` says,
/// as an operation chained to the finding (see the module's documentation).
struct FindAndScroll<F> {
    find: F,
    scroll: Id,
    decide: Decide,
}

impl<F: Operation<Option<Found>>> Operation for FindAndScroll<F> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self)
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        self.find.container(id, bounds);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        self.find.scrollable(id, bounds, content_bounds, translation, state);
    }

    fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn std::any::Any) {
        self.find.custom(id, bounds, state);
    }

    fn finish(&self) -> Outcome<()> {
        let Outcome::Some(Some(found)) = self.find.finish() else { return Outcome::None };
        match (self.decide)(found) {
            Some(y) => Outcome::Chain(Box::new(operation::scrollable::scroll_to(
                self.scroll.clone(),
                AbsoluteOffset { x: Some(0.0), y: Some(y) },
            ))),
            None => Outcome::None,
        }
    }
}

fn find_and_scroll(
    find: impl Operation<Option<Found>> + 'static,
    scroll: Id,
    decide: impl Fn(Found) -> Option<f32> + Send + 'static,
) -> Task<EditorMsg> {
    widget::operate(FindAndScroll { find, scroll, decide: Box::new(decide) }).discard()
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
    find_and_scroll(find(region_box, page.clone()), page, move |f| {
        let (top, bottom) = caret_span(&caret, f.target.width);
        offset_to_show(f.target.y + top, f.target.y + bottom, f.viewport, f.offset)
    })
}

/// Where the caret of the live widget `target` is, and the scrollable.
struct FindCaret {
    target: Id,
    scroll: Id,
    caret: Option<Rectangle>,
    scroll_state: Option<(Rectangle, Vector)>,
}

impl Operation<Option<Found>> for FindCaret {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Found>>)) {
        operate(self)
    }

    fn custom(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn std::any::Any) {
        if id == Some(&self.target)
            && let Some(c) = state.downcast_ref::<super::live_text::CaretRect>()
        {
            self.caret = Some(c.0);
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
        Outcome::Some(match (self.caret, self.scroll_state) {
            (Some(target), Some((viewport, offset))) => Some(Found { target, viewport, offset }),
            _ => None,
        })
    }
}

/// Scrolls the page so that the caret of the live widget `live` is visible.
pub fn reveal_live(live: Id, page: Id) -> Task<EditorMsg> {
    let find = FindCaret { target: live, scroll: page.clone(), caret: None, scroll_state: None };
    find_and_scroll(find, page, show_target)
}

/// Scrolls the page so that the box `target` (a table's cell) is visible.
pub fn reveal_box(target: Id, page: Id) -> Task<EditorMsg> {
    find_and_scroll(find(target, page.clone()), page, show_target)
}

/// Scrolls the page so that the box `target` is at the top.
pub fn to_top(target: Id, page: Id) -> Task<EditorMsg> {
    find_and_scroll(find(target, page.clone()), page, at_top)
}

/// Where the cell with the cursor and the table's sideways scrollable are.
struct FindCell {
    cell: Id,
    table: Id,
    cell_bounds: Option<Rectangle>,
    /// The scrollable's bounds and how far it is scrolled.
    table_state: Option<(Rectangle, Vector)>,
    /// How far the table is scrolled when it was just shown (a rendered
    /// table that became the one edited), instead of where it is now.
    from: Option<f32>,
}

impl Operation for FindCell {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self)
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.cell) {
            self.cell_bounds = Some(bounds);
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
        if id == Some(&self.table) {
            self.table_state = Some((bounds, translation));
        }
    }

    fn finish(&self) -> Outcome<()> {
        let (Some(cell), Some((viewport, offset))) = (self.cell_bounds, self.table_state) else { return Outcome::None };
        let current = self.from.unwrap_or(offset.x);
        let x = offset_x_to_show(cell.x, cell.x + cell.width, viewport, current);
        if (x - offset.x).abs() < 0.5 {
            return Outcome::None;
        }
        Outcome::Chain(Box::new(operation::scrollable::scroll_to(
            self.table.clone(),
            AbsoluteOffset { x: Some(x), y: None },
        )))
    }
}

/// The sideways offset of a table scrolled `current` far that shows the
/// cell from `left` to `right` (in the table's coordinates, unscrolled):
/// `current` if the cell is in view, else just far enough to show it.
fn offset_x_to_show(left: f32, right: f32, viewport: Rectangle, current: f32) -> f32 {
    let visible_left = viewport.x + current;
    if left < visible_left {
        (left - viewport.x).max(0.0)
    } else if right > visible_left + viewport.width {
        (right - viewport.x - viewport.width).max(0.0)
    } else {
        current
    }
}

/// Scrolls the table `table` sideways so that the cell `cell` is in view,
/// starting from the offset `from` if the table was just shown.
pub fn reveal_cell(cell: Id, table: Id, from: Option<f32>) -> Task<EditorMsg> {
    widget::operate(FindCell { cell, table, cell_bounds: None, table_state: None, from }).discard()
}

/// The offset that shows the whole of what was found, if it isn't shown.
fn show_target(f: Found) -> Option<f32> {
    offset_to_show(f.target.y, f.target.y + f.target.height, f.viewport, f.offset)
}

/// The offset that puts what was found at the top.
fn at_top(f: Found) -> Option<f32> {
    Some((f.target.y - f.viewport.y - MARGIN / 2.0).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;
    use iced::widget::scrollable::RelativeOffset;

    /// A scrollable's state that records where it was scrolled to.
    #[derive(Default)]
    struct Recorded(Option<AbsoluteOffset<Option<f32>>>);

    impl Scrollable for Recorded {
        fn snap_to(&mut self, _offset: RelativeOffset<Option<f32>>) {}

        fn scroll_to(&mut self, offset: AbsoluteOffset<Option<f32>>) {
            self.0 = Some(offset);
        }

        fn scroll_by(&mut self, _offset: AbsoluteOffset, _bounds: Rectangle, _content_bounds: Rectangle) {}
    }

    /// Runs `op` over a page (the scrollable `page`) holding the box `target`,
    /// and then the operations it chains, as iced does; where the page was
    /// scrolled to.
    fn run(mut op: Box<dyn Operation>, target: &Id, page: &Id, at: Rectangle, offset: Vector) -> Option<f32> {
        let viewport = Rectangle::new(Point::new(0.0, 30.0), Size::new(800.0, 500.0));
        let content = Rectangle::new(Point::new(0.0, 30.0), Size::new(800.0, 5000.0));
        let mut state = Recorded::default();
        loop {
            op.scrollable(Some(page), viewport, content, offset, &mut state);
            op.container(Some(target), at);
            match op.finish() {
                Outcome::Chain(next) => op = next,
                _ => break,
            }
        }
        state.0.map(|o| o.y.expect("a vertical offset"))
    }

    #[test]
    fn finding_scrolls_in_the_same_operation() {
        let (target, page) = (Id::new("block"), Id::new("doc"));
        let at = Rectangle::new(Point::new(0.0, 2030.0), Size::new(600.0, 40.0));
        let to_top = || {
            Box::new(FindAndScroll {
                find: find(target.clone(), page.clone()),
                scroll: page.clone(),
                decide: Box::new(at_top),
            })
        };
        assert_eq!(run(to_top(), &target, &page, at, Vector::ZERO), Some(2000.0 - MARGIN / 2.0));
        let show = || {
            Box::new(FindAndScroll {
                find: find(target.clone(), page.clone()),
                scroll: page.clone(),
                decide: Box::new(show_target),
            })
        };
        // Below the visible part: scrolled so that it shows at the bottom.
        assert_eq!(run(show(), &target, &page, at, Vector::ZERO), Some(2070.0 - 30.0 - 500.0 + MARGIN));
        // Visible already: left where it is.
        assert_eq!(run(show(), &target, &page, at, Vector::new(0.0, 1800.0)), None);
        // The page without the box: nothing scrolls.
        assert_eq!(run(show(), &Id::new("elsewhere"), &page, at, Vector::ZERO), None);
    }

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

    #[test]
    fn a_cell_out_of_view_is_scrolled_into_view_sideways() {
        // A table at x = 40, 600 wide, scrolled 100 far: 140..740 shows.
        let viewport = Rectangle::new(Point::new(40.0, 0.0), Size::new(600.0, 300.0));
        assert_eq!(offset_x_to_show(300.0, 500.0, viewport, 100.0), 100.0);
        assert_eq!(offset_x_to_show(90.0, 200.0, viewport, 100.0), 50.0);
        assert_eq!(offset_x_to_show(700.0, 900.0, viewport, 100.0), 260.0);
        assert_eq!(offset_x_to_show(20.0, 100.0, viewport, 100.0), 0.0);
    }

    #[test]
    fn a_table_just_shown_is_scrolled_as_it_was() {
        let (cell, table) = (Id::new("cell"), Id::new("table"));
        let viewport = Rectangle::new(Point::new(40.0, 0.0), Size::new(600.0, 300.0));
        let content = Rectangle::new(Point::new(40.0, 0.0), Size::new(1500.0, 300.0));
        let scrolled = |at: Rectangle, offset: f32, from: Option<f32>| {
            let mut op: Box<dyn Operation> = Box::new(FindCell {
                cell: cell.clone(),
                table: table.clone(),
                cell_bounds: None,
                table_state: None,
                from,
            });
            let mut state = Recorded::default();
            loop {
                op.scrollable(Some(&table), viewport, content, Vector::new(offset, 0.0), &mut state);
                op.container(Some(&cell), at);
                match op.finish() {
                    Outcome::Chain(next) => op = next,
                    _ => break,
                }
            }
            state.0.map(|o| (o.x, o.y))
        };
        let at = Rectangle::new(Point::new(500.0, 0.0), Size::new(100.0, 30.0));
        // In view: nothing scrolls.
        assert_eq!(scrolled(at, 0.0, None), None);
        // Just shown at the left, but the rendered table was scrolled 300 far.
        assert_eq!(scrolled(at, 0.0, Some(300.0)), Some((Some(300.0), None)));
        // Out of view on the right.
        let right = Rectangle::new(Point::new(800.0, 0.0), Size::new(100.0, 30.0));
        assert_eq!(scrolled(right, 0.0, None), Some((Some(260.0), None)));
    }
}
