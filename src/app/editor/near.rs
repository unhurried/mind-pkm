//! Which entries of the page column get widgets: those near the visible part
//! of the page (a margin around it), and those that must have them wherever
//! they are - the region being edited (its widgets hold the focus and the
//! text being composed), a block the page is being scrolled to, and any
//! entry whose height is not known yet (its widgets measure it when they are
//! laid out). The others take their room together, as tall as they were
//! when they were last laid out, so that the page is as tall and scrolls as
//! it would with every block built.
//!
//! The heights are kept by entry key (a block's source), for a width and
//! zoom of the column: an edited block gets a key of its own and is measured
//! anew, another width or zoom measures every block again, and a block that
//! may show math or a diagram is measured again after anything was rendered.

use super::column::{Entry, EntryKey};
use super::view::{BODY_SIZE, TEXT_LINE_HEIGHT};
use crate::markdown::BlockKind;
use std::collections::HashMap;
use std::ops::Range;

/// How far around the visible part entries get widgets, at least (it is the
/// height of the visible part when that is taller).
pub const MIN_MARGIN: f32 = 400.0;

/// The heights of the entries as last laid out.
#[derive(Debug, Default)]
pub struct Heights {
    width: f32,
    zoom: f32,
    /// Counts the renders (math, diagrams) that came in.
    renders: u64,
    /// By key: the height, and the count of renders when it was measured.
    of: HashMap<EntryKey, (f32, u64)>,
}

impl Heights {
    /// The heights for a column of `width` at `zoom`: those measured at
    /// another are forgotten.
    pub fn at(&mut self, width: f32, zoom: f32) {
        if self.width != width || self.zoom != zoom {
            *self = Heights { width, zoom, renders: self.renders, of: HashMap::new() };
        }
    }

    /// Math or a diagram was rendered: the entries that may show one are
    /// measured again.
    pub fn rendered(&mut self) {
        self.renders += 1;
    }

    /// The height an entry was laid out with.
    pub fn record(&mut self, key: EntryKey, height: f32) {
        self.of.insert(key, (height, self.renders));
    }

    /// The height of the entry `key`, if it is known; `renders`: the entry
    /// may show math or a diagram.
    pub fn get(&self, key: EntryKey, renders: bool) -> Option<f32> {
        self.of.get(&key).filter(|(_, at)| !renders || *at == self.renders).map(|(h, _)| *h)
    }

    /// Forgets the heights of the entries that are gone (edited blocks
    /// leave theirs behind), once there are many more than those `shown`.
    pub fn prune<'k>(&mut self, shown: impl ExactSizeIterator<Item = &'k EntryKey>) {
        if self.of.len() > 2 * shown.len() + 64 {
            let keep: std::collections::HashSet<&EntryKey> = shown.collect();
            self.of.retain(|k, _| keep.contains(k));
        }
    }
}

/// Whether an entry may show math or a diagram, whose height changes when
/// it is rendered.
pub fn may_render(entry: &Entry, source: &str) -> bool {
    entry.kind == BlockKind::Math || entry.kind.is_mermaid() || source.contains('$')
}

/// A guess at the height of an entry's `source` in a column of `width`:
/// its lines, wrapped at an average character width.
pub fn estimate(source: &str, width: f32, zoom: f32) -> f32 {
    let size = BODY_SIZE * zoom;
    let per_line = (width / (size * 0.55)).max(1.0);
    let lines: f32 = source.split('\n').map(|l| (l.chars().count() as f32 / per_line).ceil().max(1.0)).sum();
    lines * size * TEXT_LINE_HEIGHT
}

/// What the column shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Whether each entry gets widgets.
    pub built: Vec<bool>,
    /// Where the visible part of the column may move (top, bottom) before
    /// other entries need widgets: a margin short of the built ones, and
    /// unbounded at the ends of the page.
    pub band: (f32, f32),
}

/// What the column shows of entries that take `rooms` (each its height and
/// the gap above it, and whether that is known), when `view` is the visible
/// part of the column: the entries within `margin` of it get widgets, and
/// those `keep` names and those of unknown height wherever they are.
pub fn plan(rooms: &[(f32, bool)], view: Range<f32>, margin: f32, keep: impl Fn(usize) -> bool) -> Plan {
    let near = view.start - margin..view.end + margin;
    let mut top = 0.0;
    let built = rooms
        .iter()
        .enumerate()
        .map(|(i, &(room, known))| {
            let range = top..top + room;
            top += room;
            !known || keep(i) || (range.start < near.end && range.end > near.start)
        })
        .collect();
    let half = margin / 2.0;
    let band = (
        if near.start <= 0.0 { f32::NEG_INFINITY } else { view.start - half },
        if near.end >= top { f32::INFINITY } else { view.end + half },
    );
    Plan { built, band }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN: (f32, bool) = (100.0, true);

    fn built(plan: &Plan) -> Vec<usize> {
        plan.built.iter().enumerate().filter(|(_, b)| **b).map(|(i, _)| i).collect()
    }

    #[test]
    fn the_entries_near_the_visible_part_get_widgets() {
        // 50 entries of 100, entries 20..30 in view, 200 of margin.
        let p = plan(&[KNOWN; 50], 2000.0..3000.0, 200.0, |_| false);
        assert_eq!(built(&p), (18..32).collect::<Vec<_>>());
        assert_eq!(p.band, (1900.0, 3100.0));
        // An entry touching the margin's edge stays without.
        let p = plan(&[KNOWN; 50], 2050.0..3000.0, 250.0, |_| false);
        assert_eq!(built(&p).first(), Some(&18));
    }

    #[test]
    fn some_entries_get_widgets_wherever_they_are() {
        let mut rooms = [KNOWN; 50];
        rooms[45] = (30.0, false);
        let p = plan(&rooms, 0.0..500.0, 200.0, |i| i == 40);
        assert_eq!(built(&p), [0, 1, 2, 3, 4, 5, 6, 40, 45]);
    }

    #[test]
    fn the_band_is_open_at_the_ends_of_the_page() {
        let p = plan(&[KNOWN; 50], 100.0..900.0, 400.0, |_| false);
        assert_eq!(p.band, (f32::NEG_INFINITY, 1100.0));
        let p = plan(&[KNOWN; 50], 4200.0..4900.0, 400.0, |_| false);
        assert_eq!(p.band, (4000.0, f32::INFINITY));
        // A short page: all of it.
        let p = plan(&[KNOWN; 3], 0.0..800.0, 400.0, |_| false);
        assert_eq!((built(&p), p.band), (vec![0, 1, 2], (f32::NEG_INFINITY, f32::INFINITY)));
    }

    #[test]
    fn heights_are_kept_for_a_width_zoom_and_renders() {
        let key = EntryKey::Block { source: 1, n: 0 };
        let mut h = Heights::default();
        h.at(800.0, 1.0);
        h.record(key, 42.0);
        assert_eq!((h.get(key, false), h.get(key, true)), (Some(42.0), Some(42.0)));
        // Something rendered: an entry that may show it is measured again.
        h.rendered();
        assert_eq!((h.get(key, false), h.get(key, true)), (Some(42.0), None));
        h.record(key, 50.0);
        assert_eq!(h.get(key, true), Some(50.0));
        // The same width and zoom keep them; another forgets them.
        h.at(800.0, 1.0);
        assert_eq!(h.get(key, false), Some(50.0));
        h.at(700.0, 1.0);
        assert_eq!(h.get(key, false), None);
    }

    #[test]
    fn heights_of_gone_entries_are_forgotten_once_they_pile_up() {
        let mut h = Heights::default();
        for source in 0..100 {
            h.record(EntryKey::Block { source, n: 0 }, 10.0);
        }
        let shown = [EntryKey::Block { source: 5, n: 0 }];
        h.prune(shown.iter());
        assert_eq!(h.of.len(), 1);
        assert_eq!(h.get(shown[0], false), Some(10.0));
    }

    #[test]
    fn an_estimate_wraps_long_lines() {
        let line = BODY_SIZE * TEXT_LINE_HEIGHT;
        assert_eq!(estimate("short", 800.0, 1.0), line);
        assert_eq!(estimate("a\nb\nc", 800.0, 1.0), 3.0 * line);
        assert!(estimate(&"word ".repeat(100), 800.0, 1.0) >= 4.0 * line);
    }
}
