//! The visible part of the map: relayout, scrolling, panning, zooming.

use super::search_panel::ROW_HEIGHT;
use super::ui::zoom_step;
use super::{App, MAP_SCROLL_ID, MapMsg, Message};
use crate::mindmap::NodeId;
use iced::mouse::ScrollDelta;
use iced::widget::{operation, scrollable};
use iced::{Point, Size, Task, Vector, window};

/// Room kept around the focused node when it is scrolled into view.
const REVEAL_PADDING: f32 = 24.0;
/// The room left around the map, in screen pixels.
const MAP_MARGIN: f32 = 40.0;
/// The height of the menu and status bars, for a viewport not measured yet.
const BARS_HEIGHT: f32 = 64.0;

/// The zoom levels, 100% among them.
pub const ZOOM_STEPS: &[f32] = &[0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];
/// Wheel movement in pixels (touchpads) that counts as one notch.
const WHEEL_PIXELS_PER_STEP: f32 = 60.0;

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
    pub(super) fn viewport(&self) -> Size {
        self.viewport_size.unwrap_or_else(|| {
            let search = if self.search.open { self.search_panel_height() } else { 0.0 };
            Size::new(self.window_size.width, (self.window_size.height - BARS_HEIGHT - search).max(100.0))
        })
    }

    /// Recomputes the layout.
    pub(super) fn relayout(&mut self) {
        let Some(doc) = &mut self.doc else { return };
        doc.relayout(Size::new(MAP_MARGIN, MAP_MARGIN), self.zoom, &mut self.measure_cache);
        self.edges.clear();
    }

    /// Sets the zoom level; the measured text sizes are for the old one.
    /// Returns whether it changed.
    pub(super) fn set_zoom(&mut self, zoom: f32) -> bool {
        if zoom == self.zoom {
            return false;
        }
        self.zoom = zoom;
        self.measure_cache.clear();
        true
    }

    /// Ctrl+wheel: zooms by the notches turned (pixel deltas are added up),
    /// keeping the map point at `at` (in the viewport) in place.
    pub(super) fn zoom_wheel(&mut self, delta: ScrollDelta, at: Point) -> Task<Message> {
        let notches = match delta {
            ScrollDelta::Lines { y, .. } => y,
            ScrollDelta::Pixels { y, .. } => y / WHEEL_PIXELS_PER_STEP,
        };
        if notches * self.wheel_zoom < 0.0 {
            self.wheel_zoom = 0.0;
        }
        self.wheel_zoom += notches;
        let steps = self.wheel_zoom.trunc();
        self.wheel_zoom -= steps;
        let old = self.zoom;
        if self.doc.is_none() || !self.set_zoom(zoom_step(ZOOM_STEPS, old, steps as i32)) {
            return Task::none();
        }
        // The layout is the margin plus the map scaled by the zoom.
        let margin = Vector::new(MAP_MARGIN, MAP_MARGIN);
        self.relayout();
        let point = self.scroll_offset + Vector::new(at.x, at.y) - margin;
        let moved = margin + point * (self.zoom / old) - Vector::new(at.x, at.y);
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
        self.scroll_offset += delta;
        operation::scroll_by(MAP_SCROLL_ID, scrollable::AbsoluteOffset { x: delta.x, y: delta.y })
    }

    pub(super) fn scroll_to(&mut self, offset: Vector) -> Task<Message> {
        self.scroll_offset = offset;
        operation::scroll_to(MAP_SCROLL_ID, scrollable::AbsoluteOffset { x: offset.x, y: offset.y })
    }

    pub(super) fn center_on_focus(&mut self) -> Task<Message> {
        let Some(r) = self.focus_rect() else { return Task::none() };
        let vp = self.viewport();
        self.scroll_to(Vector::new((r.center_x() - vp.width / 2.0).max(0.0), (r.center_y() - vp.height / 2.0).max(0.0)))
    }

    /// Scrolls just enough to bring the focused node into view.
    pub(super) fn reveal_focus(&mut self) -> Task<Message> {
        let Some(r) = self.focus_rect() else { return Task::none() };
        let vp = self.viewport();
        let o = self.scroll_offset;
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

    pub(super) fn map_scrolled(&mut self, viewport: scrollable::Viewport) {
        let o = viewport.absolute_offset();
        self.scroll_offset = Vector::new(o.x, o.y);
        let size = viewport.bounds().size();
        if self.viewport_size != Some(size) {
            self.viewport_size = Some(size);
            self.relayout();
        }
    }

    pub(super) fn window_resized(&mut self, size: Size) -> Task<Message> {
        self.window_size = size;
        self.viewport_size = None;
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
        self.pan = Some(Pan::Pressed);
    }

    pub(super) fn pan_move(&mut self, position: Point) -> Task<Message> {
        match self.pan {
            None => Task::none(),
            Some(Pan::Pressed) => {
                self.pan = Some(Pan::Dragging { anchor: position, start: self.scroll_offset });
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
        self.pan = None;
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
