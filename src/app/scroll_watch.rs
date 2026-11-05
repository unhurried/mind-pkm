//! Where a scrollable is scrolled to, told only when it matters. Wrapped
//! around a scrollable, it tells the visible part of its content when that
//! leaves a band - once for each band, after any event, a redraw included,
//! so that a scroll by the wheel, the scroll bar or an operation (keeping the
//! cursor in view) is told before the window is drawn. A message for every
//! scroll would build every window again for every step of the wheel.
//!
//! The page editor builds the blocks near the visible part of the page, and
//! is told when the visible part comes near the end of what it built.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree, operation, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// The visible part of a scrollable's content: how far it is scrolled and
/// how tall the scrollable is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Visible {
    pub top: f32,
    pub height: f32,
}

pub struct ScrollWatch<'a, M> {
    content: Element<'a, M>,
    /// The scrollable watched, in `content`.
    id: widget::Id,
    /// Where the visible part may be (top, bottom) without being told.
    band: (f32, f32),
    on_leave: Box<dyn Fn(Visible) -> M + 'a>,
}

/// The band the visible part was last told for leaving.
struct Told(Option<(f32, f32)>);

/// Watches the scrollable `id` in `content`: `on_leave` is told the visible
/// part when it is not within `band` (top, bottom).
pub fn scroll_watch<'a, M>(
    content: impl Into<Element<'a, M>>,
    id: widget::Id,
    band: (f32, f32),
    on_leave: impl Fn(Visible) -> M + 'a,
) -> ScrollWatch<'a, M> {
    ScrollWatch { content: content.into(), id, band, on_leave: Box::new(on_leave) }
}

/// Finds where the scrollable `id` is scrolled, without looking into it.
struct Find {
    id: widget::Id,
    found: Option<Visible>,
}

impl Operation for Find {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if self.found.is_none() {
            operate(self);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&self.id) {
            self.found = Some(Visible { top: translation.y, height: bounds.height });
        }
    }
}

impl<M> Widget<M, Theme, Renderer> for ScrollWatch<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Told>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Told(None))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let content = &mut tree.children[0];
        self.content.as_widget_mut().update(content, event, layout, cursor, renderer, clipboard, shell, viewport);
        let mut find = Find { id: self.id.clone(), found: None };
        self.content.as_widget_mut().operate(content, layout, renderer, &mut find);
        let Some(visible) = find.found else { return };
        let Told(told) = tree.state.downcast_mut::<Told>();
        let within = self.band.0 <= visible.top && visible.top + visible.height <= self.band.1;
        if !within && *told != Some(self.band) {
            *told = Some(self.band);
            shell.publish((self.on_leave)(visible));
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, M: 'a> From<ScrollWatch<'a, M>> for Element<'a, M> {
    fn from(widget: ScrollWatch<'a, M>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::headless_renderer;
    use iced::advanced::clipboard;
    use iced::widget::scrollable::AbsoluteOffset;
    use iced::widget::{column, scrollable, space};
    use iced::{Point, window};
    use iced_runtime::user_interface::{Cache, UserInterface};

    /// A scrollable 300 tall over 3000 of content, watched for `band`.
    fn page<'a>(band: (f32, f32)) -> Element<'a, Visible> {
        let content = column((0..30).map(|_| space().height(100).into()));
        let s = scrollable(content).id(widget::Id::new("page")).width(400).height(300);
        scroll_watch(s, widget::Id::new("page"), band, |v| v).into()
    }

    /// The messages of `events`, with the window rebuilt from `cache` for `band`.
    fn told(cache: &mut Option<Cache>, band: (f32, f32), events: &[Event]) -> Vec<Visible> {
        let mut renderer = headless_renderer();
        let mut ui =
            UserInterface::build(page(band), Size::new(400.0, 300.0), cache.take().unwrap_or_default(), &mut renderer);
        let mut messages = Vec::new();
        let cursor = mouse::Cursor::Available(Point::new(100.0, 100.0));
        ui.update(events, cursor, &mut renderer, &mut clipboard::Null, &mut messages);
        *cache = Some(ui.into_cache());
        messages
    }

    fn wheel(lines: f32) -> Event {
        Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -lines * 100.0 } })
    }

    #[test]
    fn the_visible_part_is_told_when_it_leaves_the_band_once() {
        let mut cache = None;
        let band = (f32::NEG_INFINITY, 1000.0);
        // Down to 500..800: within.
        assert_eq!(told(&mut cache, band, &[wheel(5.0)]), []);
        // To 800..1100: out, told once.
        assert_eq!(told(&mut cache, band, &[wheel(3.0)]), [Visible { top: 800.0, height: 300.0 }]);
        assert_eq!(told(&mut cache, band, &[wheel(1.0)]), []);
        // The band moved with it (900..1200 is in view): told again only
        // when it leaves that one.
        let band = (600.0, 1400.0);
        assert_eq!(told(&mut cache, band, &[wheel(1.0), wheel(-4.0)]), []);
        assert_eq!(told(&mut cache, band, &[wheel(-1.0)]), [Visible { top: 500.0, height: 300.0 }]);
    }

    #[test]
    fn a_scroll_by_an_operation_is_told_at_the_redraw() {
        let mut cache = None;
        let band = (f32::NEG_INFINITY, 1000.0);
        assert_eq!(told(&mut cache, band, &[]), []);
        let mut renderer = headless_renderer();
        let mut ui = UserInterface::build(page(band), Size::new(400.0, 300.0), cache.take().unwrap(), &mut renderer);
        let to = AbsoluteOffset { x: 0.0, y: 2000.0 }.into();
        ui.operate(&renderer, &mut operation::scrollable::scroll_to(widget::Id::new("page"), to));
        cache = Some(ui.into_cache());
        let redraw = Event::Window(window::Event::RedrawRequested(iced::time::Instant::now()));
        assert_eq!(told(&mut cache, band, &[redraw]), [Visible { top: 2000.0, height: 300.0 }]);
    }
}
