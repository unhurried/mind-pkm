//! The visible part of the map: relayout, scrolling, panning, zooming.

use super::{App, MAP_SCROLL_ID, Message};
use crate::mindmap::NodeId;
use iced::mouse::ScrollDelta;
use iced::widget::{operation, scrollable};
use iced::{Point, Size, Task, Vector};

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

/// The zoom level `steps` steps away from `zoom`, within the limits.
pub fn zoom_step(zoom: f32, steps: i32) -> f32 {
    let last = ZOOM_STEPS.len() - 1;
    let i = ZOOM_STEPS.iter().position(|&z| z >= zoom - 0.001).unwrap_or(last);
    ZOOM_STEPS[(i as i64 + i64::from(steps)).clamp(0, last as i64) as usize]
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
    /// The size of the map's scrollable: as last measured, else estimated
    /// from the window size.
    pub(super) fn viewport(&self) -> Size {
        self.viewport_size.unwrap_or_else(|| {
            let search = if self.search.open { self.window_size.height * 0.4 } else { 0.0 };
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
        if self.doc.is_none() || !self.set_zoom(zoom_step(old, steps as i32)) {
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

    /// The focused node and where it is, for `keep_in_place` after a change.
    pub(super) fn focus_anchor(&self) -> Option<(NodeId, Point)> {
        let doc = self.doc.as_ref()?;
        Some((doc.focus, doc.focus_rect()?.position()))
    }

    /// Scrolls by as much as the anchored node moved in the layout, so that it
    /// stays in place on screen (unless the focus moved to another node).
    pub(super) fn keep_in_place(&mut self, anchor: Option<(NodeId, Point)>) -> Task<Message> {
        let Some((id, before)) = anchor else { return Task::none() };
        let Some(doc) = &self.doc else { return Task::none() };
        if doc.focus != id {
            return Task::none();
        }
        let Some(after) = doc.layout.get(id).map(|b| b.rect.position()) else { return Task::none() };
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

    pub(super) fn window_resized(&mut self, size: Size) {
        self.window_size = size;
        self.viewport_size = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_stop_at_the_limits() {
        assert_eq!(zoom_step(1.0, 1), 1.1);
        assert_eq!(zoom_step(1.0, -1), 0.9);
        assert_eq!(zoom_step(1.0, 3), 1.5);
        assert_eq!(zoom_step(0.5, -1), 0.5);
        assert_eq!(zoom_step(3.0, 1), 3.0);
        assert_eq!(zoom_step(1.25, -20), 0.5);
        assert!(ZOOM_STEPS.contains(&1.0));
    }
}
