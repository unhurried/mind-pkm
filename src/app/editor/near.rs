//! Which entries of the page column get widgets: those near the visible part
//! of the page (a margin around it), and those that must have them wherever
//! they are - the region being edited (its widgets hold the focus and the
//! text being composed) and a block the page is being scrolled to. The
//! others take their room together, as tall as they were when they were
//! last laid out, so that the page is as tall and scrolls as it would with
//! every block built.
//!
//! The heights are kept by entry key (a block's source), for a width and
//! zoom of the column: an edited block gets a key of its own and is measured
//! anew, and a block that may show math or a diagram is measured again after
//! anything was rendered. An entry whose height is not known takes a guess -
//! its height at the width and zoom it was last measured at, scaled, or one
//! from its source - and is measured a few at a time, the nearest to the
//! visible part first (`plan`'s budget, one batch for every message), so that
//! opening a page, zooming or resizing builds the blocks near the visible
//! part rather than all of them at once.
//!
//! As guesses turn into heights, the entries above the visible part grow or
//! shrink: the page scrolls with them (`shift`), so that the entry at the top
//! of the visible part stays where it is.

use super::column::{Entry, EntryKey};
use super::view::{BODY_SIZE, TEXT_LINE_HEIGHT};
use crate::markdown::BlockKind;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Range;

/// How far around the visible part entries get widgets, at least (it is the
/// height of the visible part when that is taller).
pub const MIN_MARGIN: f32 = 400.0;

/// How much source the entries measured with a message (those whose height
/// is not known, away from the visible part) hold together, at most - but at
/// least one entry.
pub const MEASURE_BUDGET: usize = 4096;

/// The heights of the entries as last laid out.
#[derive(Debug, Default)]
pub struct Heights {
    width: f32,
    zoom: f32,
    /// Counts the renders (math, diagrams) that came in.
    renders: u64,
    /// By key: the height, and the count of renders when it was measured.
    of: HashMap<EntryKey, (f32, u64)>,
    /// By key: heights measured at another width or zoom, with those.
    before: HashMap<EntryKey, Measured>,
}

/// A height measured at a width and zoom of the column.
#[derive(Debug, Clone, Copy)]
struct Measured {
    height: f32,
    width: f32,
    zoom: f32,
}

impl Heights {
    /// The heights for a column of `width` at `zoom`: those measured at
    /// another are kept for guesses only.
    pub fn at(&mut self, width: f32, zoom: f32) {
        if self.width != width || self.zoom != zoom {
            let (old_width, old_zoom) = (self.width, self.zoom);
            let measured = |(key, (height, _)): (EntryKey, (f32, u64))| {
                (key, Measured { height, width: old_width, zoom: old_zoom })
            };
            self.before.extend(std::mem::take(&mut self.of).into_iter().map(measured));
            (self.width, self.zoom) = (width, zoom);
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
        self.before.remove(&key);
    }

    /// The height of the entry `key`, if it is known; `renders`: the entry
    /// may show math or a diagram.
    pub fn get(&self, key: EntryKey, renders: bool) -> Option<f32> {
        self.of.get(&key).filter(|(_, at)| !renders || *at == self.renders).map(|(h, _)| *h)
    }

    /// A guess at the height of the entry `key` (whose height is not known)
    /// with the `source`: as tall as before a render came in, or as last
    /// measured at another width or zoom - scaled as its estimate is - or
    /// its estimate.
    pub fn guess(&self, key: EntryKey, source: &str) -> f32 {
        if let Some((height, _)) = self.of.get(&key) {
            return *height;
        }
        let estimate = estimate(source, self.width, self.zoom);
        match self.before.get(&key) {
            Some(m) => m.height * estimate / estimate_at(source, m.width, m.zoom),
            None => estimate,
        }
    }

    /// Forgets the heights of the entries that are gone (edited blocks
    /// leave theirs behind), once there are many more than those `shown`.
    pub fn prune<'k>(&mut self, shown: impl ExactSizeIterator<Item = &'k EntryKey>) {
        let limit = 2 * shown.len() + 64;
        if self.of.len() + self.before.len() > limit {
            let keep: std::collections::HashSet<&EntryKey> = shown.collect();
            self.of.retain(|k, _| keep.contains(k));
            self.before.retain(|k, _| keep.contains(k));
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

/// `estimate` for a width and zoom that may be none yet (a column never
/// laid out): those of 100%.
fn estimate_at(source: &str, width: f32, zoom: f32) -> f32 {
    if width > 0.0 && zoom > 0.0 { estimate(source, width, zoom) } else { estimate(source, 800.0, 1.0) }
}

/// The room an entry of the column takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Room {
    /// Its height with the gap above it, as last laid out or guessed.
    pub height: f32,
    /// Whether its height is known (else guessed).
    pub known: bool,
    /// How much source it holds: what measuring it costs.
    pub cost: usize,
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
    /// Some entries whose height is not known are left without widgets: to
    /// be measured with the next messages.
    pub unmeasured: bool,
}

/// What the column shows of entries that take `rooms`, when `view` is the
/// visible part of the column: the entries within `margin` of it get
/// widgets, and those `keep` names wherever they are; of the others whose
/// height is not known, the nearest to the view, as many as hold `budget`
/// of source together (at least one), to be measured.
pub fn plan(rooms: &[Room], view: Range<f32>, margin: f32, keep: impl Fn(usize) -> bool, budget: usize) -> Plan {
    let near = view.start - margin..view.end + margin;
    let mut top = 0.0;
    // How far each entry is from the visible part (0 within it).
    let mut away = Vec::with_capacity(rooms.len());
    let mut built: Vec<bool> = rooms
        .iter()
        .enumerate()
        .map(|(i, room)| {
            let range = top..top + room.height;
            top += room.height;
            away.push((view.start - range.end).max(range.start - view.end).max(0.0));
            keep(i) || (range.start < near.end && range.end > near.start)
        })
        .collect();
    let mut unknown: Vec<usize> = (0..rooms.len()).filter(|&i| !rooms[i].known && !built[i]).collect();
    unknown.sort_by(|&a, &b| away[a].total_cmp(&away[b]));
    let mut spent = 0;
    let mut measured = 0;
    for &i in &unknown {
        if measured > 0 && spent + rooms[i].cost > budget {
            break;
        }
        built[i] = true;
        spent += rooms[i].cost;
        measured += 1;
    }
    let half = margin / 2.0;
    let band = (
        if near.start <= 0.0 { f32::NEG_INFINITY } else { view.start - half },
        if near.end >= top { f32::INFINITY } else { view.end + half },
    );
    Plan { built, band, unmeasured: measured < unknown.len() }
}

/// How far the column's content has to scroll (down when positive) for the
/// entry at the top of the visible part to stay where it is, when the
/// entries laid out `before` (each its key, top and height) were laid out
/// again `after` and the visible part starts at `top`: how far the first
/// entry `anchors` takes that reaches below `top` moved - or, when it is gone
/// (an edit gave it another key), the nearest such entry above it that is
/// still there, else below it. `scaled`: the column was laid out at another
/// width or zoom, and the visible part starts as far into the entry at the
/// top as before for its height (its text is as tall as before for the zoom,
/// wrapped as before for the width).
pub fn shift<K: Copy + Eq + Hash>(
    before: &[(K, f32, f32)],
    after: &[(K, f32, f32)],
    top: f32,
    scaled: bool,
    anchors: impl Fn(&K) -> bool,
) -> f32 {
    let Some(first) = before.iter().position(|(k, y, h)| anchors(k) && y + h > top) else { return 0.0 };
    let now: HashMap<K, (f32, f32)> = after.iter().map(|&(k, y, h)| (k, (y, h))).collect();
    let moved = |&(k, y, _): &(K, f32, f32)| if anchors(&k) { now.get(&k).map(|(now, _)| now - y) } else { None };
    let (key, y, h) = before[first];
    if scaled
        && let Some(&(y_now, h_now)) = now.get(&key)
        && y <= top
        && h > 0.0
    {
        return y_now + (top - y) * h_now / h - top;
    }
    moved(&before[first])
        .or_else(|| before[..first].iter().rev().find_map(moved))
        .or_else(|| before[first + 1..].iter().find_map(moved))
        .unwrap_or(0.0)
}

/// The first entry `anchors` takes that reaches below `top` among those
/// `placed` (each its key, top and height), and how far below its top `top`
/// is.
pub fn at_top<K: Copy>(
    placed: &[(K, f32, f32)],
    top: f32,
    anchors: impl Fn(&K) -> Option<EntryKey>,
) -> Option<(EntryKey, f32)> {
    placed.iter().filter(|(_, y, h)| y + h > top).find_map(|(k, y, _)| anchors(k).map(|key| (key, top - y)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN: Room = Room { height: 100.0, known: true, cost: 100 };

    /// An entry 100 tall whose height is not known, holding `cost` of source.
    fn unknown(cost: usize) -> Room {
        Room { height: 100.0, known: false, cost }
    }

    fn built(plan: &Plan) -> Vec<usize> {
        plan.built.iter().enumerate().filter(|(_, b)| **b).map(|(i, _)| i).collect()
    }

    #[test]
    fn the_entries_near_the_visible_part_get_widgets() {
        // 50 entries of 100, entries 20..30 in view, 200 of margin.
        let p = plan(&[KNOWN; 50], 2000.0..3000.0, 200.0, |_| false, MEASURE_BUDGET);
        assert_eq!(built(&p), (18..32).collect::<Vec<_>>());
        assert_eq!(p.band, (1900.0, 3100.0));
        assert!(!p.unmeasured);
        // An entry touching the margin's edge stays without.
        let p = plan(&[KNOWN; 50], 2050.0..3000.0, 250.0, |_| false, MEASURE_BUDGET);
        assert_eq!(built(&p).first(), Some(&18));
    }

    #[test]
    fn some_entries_get_widgets_wherever_they_are() {
        let mut rooms = [KNOWN; 50];
        rooms[45] = unknown(30);
        let p = plan(&rooms, 0.0..500.0, 200.0, |i| i == 40, MEASURE_BUDGET);
        assert_eq!(built(&p), [0, 1, 2, 3, 4, 5, 6, 40, 45]);
        assert!(!p.unmeasured);
    }

    #[test]
    fn entries_of_unknown_height_are_measured_a_few_at_a_time_nearest_first() {
        // In view 20..30; unknown: 2, 10, 40 and 45 (each 1000 of source).
        let mut rooms = [KNOWN; 50];
        for i in [2, 10, 40, 45] {
            rooms[i] = unknown(1000);
        }
        let p = plan(&rooms, 2000.0..3000.0, 200.0, |_| false, 2500);
        // 10 is the nearest (900 above the view), then 40 (1000 below it).
        assert_eq!(built(&p), [10, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 40]);
        assert!(p.unmeasured);
        // Measured: the rest next time.
        rooms[10] = KNOWN;
        rooms[40] = KNOWN;
        let p = plan(&rooms, 2000.0..3000.0, 200.0, |_| false, 2500);
        assert!(built(&p).contains(&2) && built(&p).contains(&45) && !p.unmeasured);
        // One at least, however much it holds.
        let p = plan(&[unknown(100_000); 50], 0.0..100.0, 0.0, |_| false, 10);
        assert_eq!((built(&p), p.unmeasured), (vec![0, 1], true));
    }

    #[test]
    fn the_band_is_open_at_the_ends_of_the_page() {
        let p = plan(&[KNOWN; 50], 100.0..900.0, 400.0, |_| false, MEASURE_BUDGET);
        assert_eq!(p.band, (f32::NEG_INFINITY, 1100.0));
        let p = plan(&[KNOWN; 50], 4200.0..4900.0, 400.0, |_| false, MEASURE_BUDGET);
        assert_eq!(p.band, (4000.0, f32::INFINITY));
        // A short page: all of it.
        let p = plan(&[KNOWN; 3], 0.0..800.0, 400.0, |_| false, MEASURE_BUDGET);
        assert_eq!((built(&p), p.band), (vec![0, 1, 2], (f32::NEG_INFINITY, f32::INFINITY)));
    }

    #[test]
    fn heights_at_another_width_or_zoom_are_guesses() {
        let key = EntryKey::Block { source: 1, n: 0 };
        let source = "word ".repeat(300);
        let mut h = Heights::default();
        h.at(800.0, 1.0);
        h.record(key, 500.0);
        // Twice the zoom on a column twice as wide: the same lines, twice as tall.
        h.at(1600.0, 2.0);
        assert_eq!(h.get(key, false), None);
        assert!((h.guess(key, &source) - 1000.0).abs() < 1.0, "{}", h.guess(key, &source));
        // Half as wide: about twice as many lines.
        h.at(400.0, 1.0);
        let narrow = h.guess(key, &source);
        assert!((900.0..1100.0).contains(&narrow), "{narrow}");
        // Measured again: known.
        h.record(key, 980.0);
        assert_eq!((h.get(key, false), h.guess(key, &source)), (Some(980.0), 980.0));
        // Never measured: its estimate.
        let other = EntryKey::Block { source: 2, n: 0 };
        assert_eq!(h.guess(other, "short"), estimate("short", 400.0, 1.0));
        // Measured before a render came in: as tall as then.
        h.rendered();
        assert_eq!((h.get(key, true), h.guess(key, &source)), (None, 980.0));
    }

    #[test]
    fn the_entry_at_the_top_stays_where_it_is() {
        let before = [(1, 0.0, 100.0), (2, 100.0, 50.0), (3, 150.0, 300.0), (4, 450.0, 100.0)];
        // 2 grew by 40 (it was measured, say).
        let after = [(1, 0.0, 100.0), (2, 100.0, 90.0), (3, 190.0, 300.0), (4, 490.0, 100.0)];
        // At the top: 2, which stayed where it was.
        assert_eq!(shift(&before, &after, 120.0, false, |_| true), 0.0);
        // 3, which moved down by 40.
        assert_eq!(shift(&before, &after, 160.0, false, |_| true), 40.0);
        // 2 does not anchor: 3 does.
        assert_eq!(shift(&before, &after, 120.0, false, |k| *k != 2), 40.0);
        // Gone (edited): the one above it, which stayed.
        let without_3 = [(1, 0.0, 100.0), (2, 100.0, 90.0), (4, 190.0, 100.0)];
        assert_eq!(shift(&before, &without_3, 160.0, false, |_| true), 0.0);
        let moved_down = [(1, 10.0, 100.0), (2, 110.0, 90.0), (4, 200.0, 100.0)];
        assert_eq!(shift(&before, &moved_down, 160.0, false, |_| true), 10.0);
        // At another zoom (every entry twice as tall): as far into the entry
        // at the top for its height.
        let zoomed = [(1, 0.0, 200.0), (2, 200.0, 100.0), (3, 300.0, 600.0), (4, 900.0, 200.0)];
        assert_eq!(shift(&before, &zoomed, 120.0, true, |_| true), 240.0 - 120.0);
        assert_eq!(shift(&before, &zoomed, 120.0, false, |_| true), 100.0);
        // None above it: the one below.
        assert_eq!(shift(&before[2..], &[(4, 490.0, 100.0)], 160.0, false, |_| true), 40.0);
        // Nothing laid out before or now: nothing to keep in place.
        assert_eq!(shift(&[], &after, 160.0, false, |_| true), 0.0);
        // The entry at the top, and how far into it the top is.
        let key = |k: &i32| (*k != 2).then_some(EntryKey::Block { source: *k as u64, n: 0 });
        assert_eq!(at_top(&before, 120.0, key), Some((EntryKey::Block { source: 3, n: 0 }, -30.0)));
        assert_eq!(at_top(&before, 470.0, key), Some((EntryKey::Block { source: 4, n: 0 }, 20.0)));
        assert_eq!(at_top(&before, 600.0, key), None);
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
        // The same width and zoom keep them; at another they are guesses.
        h.at(800.0, 1.0);
        assert_eq!(h.get(key, false), Some(50.0));
        h.at(700.0, 1.0);
        assert_eq!(h.get(key, false), None);
        h.at(800.0, 1.0);
        assert_eq!((h.get(key, false), h.guess(key, "x")), (None, 50.0));
    }

    #[test]
    fn heights_of_gone_entries_are_forgotten_once_they_pile_up() {
        let mut h = Heights::default();
        for source in 0..100 {
            h.record(EntryKey::Block { source, n: 0 }, 10.0);
        }
        let shown = [EntryKey::Block { source: 5, n: 0 }];
        h.at(800.0, 1.0);
        for source in 100..200 {
            h.record(EntryKey::Block { source, n: 0 }, 10.0);
        }
        h.prune(shown.iter());
        assert_eq!((h.of.len(), h.before.len()), (0, 1));
        h.record(shown[0], 10.0);
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
