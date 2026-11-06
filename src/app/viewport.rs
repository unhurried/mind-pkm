//! The visible part of the map: relayout, scrolling, panning, zooming.

use super::scroll_watch::Visible;
use super::search_panel::ROW_HEIGHT;
use super::ui::zoom_step;
use super::view;
use super::{App, MAP_SCROLL_ID, MapMsg, Message};
use crate::mindmap::NodeId;
use iced::mouse::ScrollDelta;
use iced::widget::{canvas, operation, scrollable};
use iced::{Point, Rectangle, Size, Task, Vector, window};
use std::cell::Cell;

/// Room kept around the focused node when it is scrolled into view.
const REVEAL_PADDING: f32 = 24.0;
/// The room left around the map, in screen pixels.
const MAP_MARGIN: f32 = 40.0;
/// The height of the menu and status bars, for a viewport not measured yet.
const BARS_HEIGHT: f32 = 64.0;
/// The room around the visible part of the map whose nodes are built as
/// widgets too (`near_view`), in screen pixels. The map is told where it is
/// scrolled when the visible part comes within half of it of the edge of the
/// nodes built (`near_band`).
pub(super) const NEAR_MARGIN: f32 = 400.0;

/// The zoom levels, 100% among them.
pub const ZOOM_STEPS: &[f32] = &[0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];
/// Wheel movement in pixels (touchpads) that counts as one notch.
const WHEEL_PIXELS_PER_STEP: f32 = 60.0;

/// What the map window shows of the map: the zoom, the scroll position, the
/// viewport's size, a pan in progress, the measured text sizes (at 100%,
/// scaled to the zoom) and the edges as drawn.
pub struct MapViewport {
    /// One of `ZOOM_STEPS`.
    zoom: f32,
    /// Ctrl+wheel movement not yet turned into a zoom step, in notches.
    wheel_zoom: f32,
    /// Node text sizes at 100%, by text and width.
    measure_cache: view::MeasureCache,
    /// The drawn edges, cleared on every relayout.
    pub(super) edges: canvas::Cache,
    /// The scroll position, as last scrolled to or as the scrollable is
    /// after the last event (followed without a message).
    scroll: Cell<Vector>,
    /// The scroll position the nodes near the visible part are built for:
    /// as last scrolled to or told (`MapMsg::MapScrolled`).
    built_for: Vector,
    /// Where the scrollable was after the last event: it is followed only
    /// when it moved (a scroll by the application is done after the windows
    /// are built again, and the scrollable is where it was until then).
    seen: Cell<Option<Vector>>,
    /// The size of the map's scrollable as after the last event; None when
    /// it may have changed.
    size: Cell<Option<Size>>,
    /// Set by a move among siblings: the parent stays in place on screen
    /// after the message, not the moved node.
    pub(super) keep_parent_in_place: bool,
    pub(super) pan: Option<Pan>,
}

impl Default for MapViewport {
    fn default() -> Self {
        MapViewport {
            zoom: 1.0,
            wheel_zoom: 0.0,
            measure_cache: view::MeasureCache::default(),
            edges: canvas::Cache::default(),
            scroll: Cell::new(Vector::ZERO),
            built_for: Vector::ZERO,
            seen: Cell::new(None),
            size: Cell::new(None),
            keep_parent_in_place: false,
            pan: None,
        }
    }
}

impl MapViewport {
    pub(super) fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Sets the zoom level. Returns whether it changed.
    pub(super) fn set_zoom(&mut self, zoom: f32) -> bool {
        if zoom == self.zoom {
            return false;
        }
        self.zoom = zoom;
        true
    }

    /// The scroll position: as last scrolled to, or as the scrollable is
    /// after the last event.
    pub(super) fn scroll(&self) -> Vector {
        self.scroll.get()
    }

    pub(super) fn set_scroll(&mut self, scroll: Vector) {
        self.scroll.set(scroll);
        self.built_for = scroll;
    }

    /// The size of the map's scrollable, as after the last event.
    pub(super) fn size(&self) -> Option<Size> {
        self.size.get()
    }

    /// The scrollable may change size (the window was resized, the search
    /// panel opened or closed): its size is not known until the next event.
    pub(super) fn forget_size(&self) {
        self.size.set(None);
    }

    /// Follows the map's scrollable: where it is after an event, if it moved
    /// since the last one (until a scroll the application asked for is done,
    /// the scroll position is that one), and its size. The nodes built stay
    /// those near where it was last told or scrolled to.
    pub(super) fn follow(&self, visible: Visible) {
        let at = Vector::new(visible.left, visible.top);
        if self.seen.replace(Some(at)) != Some(at) {
            self.scroll.set(at);
        }
        self.size.set(Some(Size::new(visible.width, visible.height)));
    }

    /// The size of a node's text at the current zoom, as measured; `wide`: for
    /// a node below ToDo, which wraps at a larger width.
    pub(super) fn measured(&self, text: &str, wide: bool) -> Option<Size> {
        self.measure_cache.get(text, wide).map(|size| view::text_at_zoom(size, self.zoom))
    }
}

/// Panning by dragging empty map space.
#[derive(Debug, Clone, Copy)]
pub enum Pan {
    /// The button went down; the drag is anchored at the first move.
    Pressed,
    /// The pointer position the drag is anchored at and the scroll offset at that moment.
    Dragging { anchor: Point, start: Vector },
}

impl App {
    /// The height of the search panel: two fifths of the room between the
    /// bars, less two result rows.
    pub(super) fn search_panel_height(&self) -> f32 {
        let body = (self.window_size.height - BARS_HEIGHT).max(0.0);
        (body * 0.4 - 2.0 * ROW_HEIGHT).max(0.0)
    }

    /// The size of the map's scrollable: as last measured, else estimated
    /// from the window size.
    pub(super) fn viewport_size(&self) -> Size {
        self.viewport.size().unwrap_or_else(|| {
            let search = if self.search.open { self.search_panel_height() } else { 0.0 };
            Size::new(self.window_size.width, (self.window_size.height - BARS_HEIGHT - search).max(100.0))
        })
    }

    /// The part of the map whose nodes are shown as widgets, in the map's
    /// coordinates: the visible part at the scroll position last told or
    /// scrolled to (inside the map of `map_size`), with `NEAR_MARGIN` around
    /// it - the visible part is within half of that of it.
    pub(super) fn near_view(&self, map_size: Size) -> Rectangle {
        let view = self.viewport_size();
        let max = Vector::new((map_size.width - view.width).max(0.0), (map_size.height - view.height).max(0.0));
        let scroll = self.viewport.built_for;
        let at = Point::new(scroll.x.clamp(0.0, max.x) - NEAR_MARGIN, scroll.y.clamp(0.0, max.y) - NEAR_MARGIN);
        Rectangle::new(at, Size::new(view.width + 2.0 * NEAR_MARGIN, view.height + 2.0 * NEAR_MARGIN))
    }

    /// Where the visible part of the map may move without other nodes
    /// needing widgets, the nodes `near` it built (left, right; top, bottom):
    /// half the margin short of those.
    pub(super) fn near_band(near: Rectangle) -> ((f32, f32), (f32, f32)) {
        let inset = NEAR_MARGIN / 2.0;
        ((near.x + inset, near.x + near.width - inset), (near.y + inset, near.y + near.height - inset))
    }

    /// Recomputes the layout if anything it depends on changed; the edges
    /// are drawn again only then.
    pub(super) fn relayout(&mut self) {
        let draft = self.draft();
        let Some(doc) = &mut self.doc else { return };
        if doc.relayout(Size::new(MAP_MARGIN, MAP_MARGIN), self.viewport.zoom, &mut self.viewport.measure_cache, draft)
        {
            self.viewport.edges.clear();
        }
    }

    /// Ctrl+wheel: zooms by the notches turned (pixel deltas are added up),
    /// keeping the map point at `at` (in the viewport) in place.
    pub(super) fn zoom_wheel(&mut self, delta: ScrollDelta, at: Point) -> Task<Message> {
        let notches = match delta {
            ScrollDelta::Lines { y, .. } => y,
            ScrollDelta::Pixels { y, .. } => y / WHEEL_PIXELS_PER_STEP,
        };
        if notches * self.viewport.wheel_zoom < 0.0 {
            self.viewport.wheel_zoom = 0.0;
        }
        self.viewport.wheel_zoom += notches;
        let steps = self.viewport.wheel_zoom.trunc();
        self.viewport.wheel_zoom -= steps;
        let old = self.viewport.zoom;
        if self.doc.is_none() || !self.viewport.set_zoom(zoom_step(ZOOM_STEPS, old, steps as i32)) {
            return Task::none();
        }
        // The layout is the margin plus the map scaled by the zoom.
        let margin = Vector::new(MAP_MARGIN, MAP_MARGIN);
        self.relayout();
        let point = self.viewport.scroll() + Vector::new(at.x, at.y) - margin;
        let moved = margin + point * (self.viewport.zoom / old) - Vector::new(at.x, at.y);
        self.scroll_to(Vector::new(moved.x.max(0.0), moved.y.max(0.0)))
    }

    pub(super) fn focus_rect(&self) -> Option<iced::Rectangle> {
        self.doc.as_ref()?.focus_rect()
    }

    /// The focused node, the node to keep in place after a change (the focused
    /// node or its parent) and where that one is, for `keep_in_place`.
    pub(super) fn focus_anchor(&self, parent: bool) -> Option<(NodeId, NodeId, Point)> {
        let doc = self.doc.as_ref()?;
        let id = if parent { doc.map().parent(doc.focus())? } else { doc.focus() };
        Some((doc.focus(), id, doc.layout().get(id)?.rect.position()))
    }

    /// Scrolls by as much as the anchored node moved in the layout, so that it
    /// stays in place on screen (unless the focus moved to another node).
    pub(super) fn keep_in_place(&mut self, anchor: Option<(NodeId, NodeId, Point)>) -> Task<Message> {
        let Some((focus, id, before)) = anchor else { return Task::none() };
        let Some(doc) = &self.doc else { return Task::none() };
        if doc.focus() != focus {
            return Task::none();
        }
        let Some(after) = doc.layout().get(id).map(|b| b.rect.position()) else { return Task::none() };
        let delta = after - before;
        if delta == Vector::ZERO {
            return Task::none();
        }
        self.viewport.set_scroll(self.viewport.scroll() + delta);
        operation::scroll_by(MAP_SCROLL_ID, scrollable::AbsoluteOffset { x: delta.x, y: delta.y })
    }

    pub(super) fn scroll_to(&mut self, offset: Vector) -> Task<Message> {
        self.viewport.set_scroll(offset);
        operation::scroll_to(MAP_SCROLL_ID, scrollable::AbsoluteOffset { x: offset.x, y: offset.y })
    }

    pub(super) fn center_on_focus(&mut self) -> Task<Message> {
        let Some(r) = self.focus_rect() else { return Task::none() };
        let vp = self.viewport_size();
        self.scroll_to(Vector::new((r.center_x() - vp.width / 2.0).max(0.0), (r.center_y() - vp.height / 2.0).max(0.0)))
    }

    /// Scrolls just enough to bring the focused node into view.
    pub(super) fn reveal_focus(&mut self) -> Task<Message> {
        let Some(r) = self.focus_rect() else { return Task::none() };
        let vp = self.viewport_size();
        let o = self.viewport.scroll();
        let axis = |start: f32, len: f32, off: f32, view: f32| {
            if start - REVEAL_PADDING < off {
                (start - REVEAL_PADDING).max(0.0)
            } else if start + len + REVEAL_PADDING > off + view {
                start + len + REVEAL_PADDING - view
            } else {
                off
            }
        };
        let new = Vector::new(axis(r.x, r.width, o.x, vp.width), axis(r.y, r.height, o.y, vp.height));
        if new == o { Task::none() } else { self.scroll_to(new) }
    }

    /// The visible part of the map left the nodes built around it: the view
    /// is built again (for where it is now).
    pub(super) fn map_scrolled(&mut self, visible: Visible) {
        self.viewport.follow(visible);
        self.viewport.built_for = self.viewport.scroll();
    }

    pub(super) fn window_resized(&mut self, size: Size) -> Task<Message> {
        self.window_size = size;
        self.viewport.forget_size();
        ask_window_state(self.main_window)
    }

    pub(super) fn window_moved(&mut self, position: Point) -> Task<Message> {
        self.window_position = Some(position);
        ask_window_state(self.main_window)
    }

    /// Takes the window's size and position as the ones to restore, unless it
    /// is maximized (then only that is noted) or minimized.
    pub(super) fn window_state(&mut self, maximized: bool, minimized: bool) {
        if minimized || self.window_size.width < 1.0 || self.window_size.height < 1.0 {
            return;
        }
        self.window.maximized = maximized;
        if !maximized {
            self.window.size = self.window_size;
            self.window.position = self.window_position.or(self.window.position);
        }
    }

    pub(super) fn pan_start(&mut self) {
        self.menu = None;
        self.viewport.pan = Some(Pan::Pressed);
    }

    pub(super) fn pan_move(&mut self, position: Point) -> Task<Message> {
        match self.viewport.pan {
            None => Task::none(),
            Some(Pan::Pressed) => {
                self.viewport.pan = Some(Pan::Dragging { anchor: position, start: self.viewport.scroll() });
                Task::none()
            }
            // The map moves with the pointer; the scrollable clamps the far edges.
            Some(Pan::Dragging { anchor, start }) => self.scroll_to(Vector::new(
                (start.x - (position.x - anchor.x)).max(0.0),
                (start.y - (position.y - anchor.y)).max(0.0),
            )),
        }
    }

    pub(super) fn pan_end(&mut self) {
        self.viewport.pan = None;
    }
}

/// Asks whether the map window is maximized or minimized now.
fn ask_window_state(id: window::Id) -> Task<Message> {
    window::is_maximized(id).then(move |maximized| {
        window::is_minimized(id)
            .map(move |minimized| MapMsg::WindowState { maximized, minimized: minimized.unwrap_or(false) }.into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible(left: f32, top: f32) -> Visible {
        Visible { top, height: 600.0, left, width: 800.0 }
    }

    #[test]
    fn the_scroll_position_follows_the_scrollable_when_it_moves() {
        let mut v = MapViewport::default();
        v.follow(visible(0.0, 100.0));
        assert_eq!((v.scroll(), v.size()), (Vector::new(0.0, 100.0), Some(Size::new(800.0, 600.0))));
        // Scrolled by the application: the scrollable, not there yet (the
        // scroll is done after the windows are built again), leaves it so.
        v.set_scroll(Vector::new(50.0, 500.0));
        v.follow(visible(0.0, 100.0));
        assert_eq!(v.scroll(), Vector::new(50.0, 500.0));
        // There (as far as it goes): followed.
        v.follow(visible(40.0, 500.0));
        assert_eq!(v.scroll(), Vector::new(40.0, 500.0));
        // Scrolled by the wheel.
        v.follow(visible(40.0, 560.0));
        assert_eq!(v.scroll(), Vector::new(40.0, 560.0));
    }

    #[test]
    fn zoom_steps_stop_at_the_limits() {
        assert_eq!(zoom_step(ZOOM_STEPS, 1.0, 1), 1.1);
        assert_eq!(zoom_step(ZOOM_STEPS, 1.0, -1), 0.9);
        assert_eq!(zoom_step(ZOOM_STEPS, 1.0, 3), 1.5);
        assert_eq!(zoom_step(ZOOM_STEPS, 0.5, -1), 0.5);
        assert_eq!(zoom_step(ZOOM_STEPS, 3.0, 1), 3.0);
        assert_eq!(zoom_step(ZOOM_STEPS, 1.25, -20), 0.5);
        assert!(ZOOM_STEPS.contains(&1.0));
    }
}
