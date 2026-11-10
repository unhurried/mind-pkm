//! Where a scrollable is scrolled to, told only when it matters. Wrapped
//! around a scrollable, it tells the visible part of its content when that
//! leaves a band (above and below it, and left and right of it) - once for
//! each band, after any event, a redraw included, so that a scroll by the
//! wheel, the scroll bar or an operation (keeping the cursor in view) is told
//! before the window is drawn. A message for every scroll would build every
//! window again for every step of the wheel. Where it is after every event
//! can be followed too, without a message (`tracking`).
//!
//! The page editor builds the blocks near the visible part of the page, and
//! is told when the visible part comes near the end of what it built; so
//! does the map with its nodes, which follows the scroll position as well
//! (zooming and panning start from it).
//!
//! It can also keep what is at the top of the visible part in place when the
//! content is laid out anew (`anchor`): asked how far, it scrolls the content
//! right after laying it out, before it is drawn - the page editor's blocks
//! above the visible part change height as they are measured.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree, operation, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// The visible part of a scrollable's content: how far it is scrolled down
/// and right, and how tall and wide the scrollable is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Visible {
    pub top: f32,
    pub height: f32,
    pub left: f32,
    pub width: f32,
}

pub struct ScrollWatch<'a, M> {
    content: Element<'a, M>,
    /// The scrollable watched, in `content`.
    id: widget::Id,
    /// Where the visible part may be (top, bottom) without being told.
    band: (f32, f32),
    /// Where it may be (left, right).
    band_x: (f32, f32),
    on_leave: Box<dyn Fn(Visible) -> M + 'a>,
    tracking: Option<Box<dyn Fn(Visible) + 'a>>,
    track_layout: bool,
    anchor: Option<Box<dyn Fn(Visible) -> f32 + 'a>>,
}

/// The bands the visible part was last told for leaving.
struct Told(Option<((f32, f32), (f32, f32))>);

/// Watches the scrollable `id` in `content`: `on_leave` is told the visible
/// part when it is not within `band` (top, bottom).
pub fn scroll_watch<'a, M>(
    content: impl Into<Element<'a, M>>,
    id: widget::Id,
    band: (f32, f32),
    on_leave: impl Fn(Visible) -> M + 'a,
) -> ScrollWatch<'a, M> {
    ScrollWatch {
        content: content.into(),
        id,
        band,
        band_x: (f32::NEG_INFINITY, f32::INFINITY),
        on_leave: Box::new(on_leave),
        tracking: None,
        track_layout: false,
        anchor: None,
    }
}

impl<'a, M> ScrollWatch<'a, M> {
    /// Where the visible part may be (left, right) without being told; any
    /// where when not given.
    pub fn band_x(mut self, band: (f32, f32)) -> Self {
        self.band_x = band;
        self
    }

    /// Tells `f` the visible part after every event, without a message.
    pub fn tracking(mut self, f: impl Fn(Visible) + 'a) -> Self {
        self.tracking = Some(Box::new(f));
        self
    }

    /// Like `tracking`, also immediately after layout and anchoring, so
    /// drawing can use the current offset even before the next event.
    pub fn tracking_layout(mut self, f: impl Fn(Visible) + 'a) -> Self {
        self.tracking = Some(Box::new(f));
        self.track_layout = true;
        self
    }

    /// Keeps what is at the top of the visible part in place: whenever the
    /// content was laid out, `f` is told the visible part (as scrolled
    /// before) and says how far to scroll the content (down when positive).
    pub fn anchor(mut self, f: impl Fn(Visible) -> f32 + 'a) -> Self {
        self.anchor = Some(Box::new(f));
        self
    }
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
            let (top, left) = (translation.y, translation.x);
            self.found = Some(Visible { top, height: bounds.height, left, width: bounds.width });
        }
    }
}

/// Scrolls the scrollable `id` by `by` (down when positive), without looking
/// into it.
struct ScrollBy {
    id: widget::Id,
    by: f32,
    done: bool,
}

impl Operation for ScrollBy {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if !self.done {
            operate(self);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&self.id) {
            state.scroll_by(AbsoluteOffset { x: 0.0, y: self.by }, bounds, content_bounds);
            self.done = true;
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
        let content = &mut tree.children[0];
        let node = self.content.as_widget_mut().layout(content, renderer, limits);
        if let Some(anchor) = &self.anchor {
            let layout = Layout::new(&node);
            let mut find = Find { id: self.id.clone(), found: None };
            self.content.as_widget_mut().operate(content, layout, renderer, &mut find);
            if let Some(visible) = find.found {
                let by = anchor(visible);
                if by != 0.0 {
                    let mut scroll = ScrollBy { id: self.id.clone(), by, done: false };
                    self.content.as_widget_mut().operate(content, layout, renderer, &mut scroll);
                }
            }
        }
        // Layout (including the anchor above) can change the actual offset
        // without an event. Drawing must use that offset immediately too.
        if self.track_layout
            && let Some(tracking) = &self.tracking
        {
            let mut find = Find { id: self.id.clone(), found: None };
            self.content.as_widget_mut().operate(content, Layout::new(&node), renderer, &mut find);
            if let Some(visible) = find.found {
                tracking(visible);
            }
        }
        node
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
        if let Some(tracking) = &self.tracking {
            tracking(visible);
        }
        let Told(told) = tree.state.downcast_mut::<Told>();
        let within = |(start, end): (f32, f32), at: f32, len: f32| start <= at && at + len <= end;
        let within = within(self.band, visible.top, visible.height) && within(self.band_x, visible.left, visible.width);
        let bands = (self.band, self.band_x);
        if !within && *told != Some(bands) {
            *told = Some(bands);
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

    /// The page scrolled down to `top`.
    fn visible(top: f32) -> Visible {
        Visible { top, height: 300.0, left: 0.0, width: 400.0 }
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
        assert_eq!(told(&mut cache, band, &[wheel(3.0)]), [visible(800.0)]);
        assert_eq!(told(&mut cache, band, &[wheel(1.0)]), []);
        // The band moved with it (900..1200 is in view): told again only
        // when it leaves that one.
        let band = (600.0, 1400.0);
        assert_eq!(told(&mut cache, band, &[wheel(1.0), wheel(-4.0)]), []);
        assert_eq!(told(&mut cache, band, &[wheel(-1.0)]), [visible(500.0)]);
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
        assert_eq!(told(&mut cache, band, &[redraw]), [visible(2000.0)]);
    }

    /// A scrollable 300 tall over a first child `first` tall and 29 of 100,
    /// kept in place by `anchor`.
    fn anchored<'a>(first: f32, anchor: impl Fn(Visible) -> f32 + 'a) -> Element<'a, Visible> {
        let children = std::iter::once(space().height(first).into()).chain((0..29).map(|_| space().height(100).into()));
        let s = scrollable(column(children)).id(widget::Id::new("page")).width(400).height(300);
        scroll_watch(s, widget::Id::new("page"), (f32::NEG_INFINITY, f32::INFINITY), |v| v).anchor(anchor).into()
    }

    /// Where the scrollable of `ui` is scrolled to.
    fn scrolled(ui: &mut UserInterface<'_, Visible, Theme, Renderer>, renderer: &Renderer) -> f32 {
        let mut find = Find { id: widget::Id::new("page"), found: None };
        ui.operate(renderer, &mut find);
        find.found.expect("the scrollable").top
    }

    #[test]
    fn the_content_is_scrolled_as_the_anchor_says_when_laid_out_anew() {
        let mut renderer = headless_renderer();
        let size = Size::new(400.0, 300.0);
        let mut ui = UserInterface::build(anchored(100.0, |_| 0.0), size, Cache::default(), &mut renderer);
        let to = AbsoluteOffset { x: 0.0, y: 500.0 }.into();
        ui.operate(&renderer, &mut operation::scrollable::scroll_to(widget::Id::new("page"), to));
        assert_eq!(scrolled(&mut ui, &renderer), 500.0);
        // The first child grew by 200: scrolled with it as it is laid out,
        // the anchor told where the page was.
        let told = std::cell::Cell::new(None);
        let anchor = |v: Visible| {
            told.set(Some(v));
            200.0
        };
        let mut ui = UserInterface::build(anchored(300.0, anchor), size, ui.into_cache(), &mut renderer);
        assert_eq!(told.get(), Some(visible(500.0)));
        assert_eq!(scrolled(&mut ui, &renderer), 700.0);
        // Nothing moved: it stays.
        let mut ui = UserInterface::build(anchored(300.0, |_| 0.0), size, ui.into_cache(), &mut renderer);
        assert_eq!(scrolled(&mut ui, &renderer), 700.0);
    }

    #[test]
    fn tracking_sees_the_anchored_offset_before_drawing_without_an_event() {
        let mut renderer = headless_renderer();
        let tracked = std::cell::Cell::new(None);
        let content = scrollable(space().height(3000)).id(widget::Id::new("page")).width(400).height(300);
        let watched = scroll_watch(content, widget::Id::new("page"), (0.0, 3000.0), |v| v)
            .anchor(|_| 200.25)
            .tracking_layout(|v| tracked.set(Some(v)));
        let mut ui = UserInterface::build(watched, Size::new(400.0, 300.0), Cache::default(), &mut renderer);
        // iced rounds the scroll translation used to draw the content;
        // tracking needs that effective offset, not the requested one.
        assert_eq!(scrolled(&mut ui, &renderer), 200.0);
        assert_eq!(tracked.get(), Some(visible(200.0)));
    }
}
