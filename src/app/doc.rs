//! An open map with its view state: the selection, the filter, the layout and
//! the undo history, and the edits that only concern the map.

use super::layout::{self, MapLayout};
use super::view;
use crate::mindmap::{MindMap, NodeId};
use crate::pkm::{self, commands, filter::Filter};
use iced::{Rectangle, Size};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

const UNDO_LIMIT: usize = 100;

pub struct Doc {
    /// Distinguishes this document from a later one in background results.
    pub(super) generation: u64,
    pub(super) map: MindMap,
    /// The selected nodes, in the order they were selected.
    pub(super) selection: Vec<NodeId>,
    /// The node single-node commands act on; always among the selection.
    pub(super) focus: NodeId,
    pub(super) filter: Filter,
    pub(super) layout: MapLayout,
    /// Changed since last saved.
    pub(super) dirty: bool,
    /// When the last Next Steps / search index refresh started, and whether
    /// the map has been edited since.
    pub(super) last_refresh: Instant,
    pub(super) edited_since_refresh: bool,
    undo: VecDeque<Checkpoint>,
    redo: Vec<Selected>,
    /// The serial number of the last checkpoint.
    serial: u64,
    /// The map as last laid out: where a selected node that is gone was.
    shown: MindMap,
}

/// The map as it was before a change, with the serial number identifying it
/// and the unsaved / edited marks it had.
struct Checkpoint {
    serial: u64,
    state: Selected,
    dirty: bool,
    edited: bool,
}

/// A map with the selection it had.
struct Selected {
    map: MindMap,
    selection: Vec<NodeId>,
    focus: NodeId,
}

impl Doc {
    pub(super) fn new(map: MindMap, generation: u64) -> Self {
        let root = map.root();
        Doc {
            generation,
            shown: map.clone(),
            map,
            selection: vec![root],
            focus: root,
            filter: Filter::HideArchive,
            layout: MapLayout::default(),
            dirty: false,
            last_refresh: Instant::now(),
            edited_since_refresh: false,
            undo: VecDeque::new(),
            redo: Vec::new(),
            serial: 0,
        }
    }

    // --- Undo ---

    /// Records the current map for undo; call before every change. Returns the
    /// checkpoint's serial number.
    pub(super) fn checkpoint(&mut self) -> u64 {
        self.record(self.map.clone())
    }

    /// Records `before` as the map preceding a change just made (with the
    /// selection, which the change hasn't touched yet). Returns the
    /// checkpoint's serial number.
    pub(super) fn record(&mut self, before: MindMap) -> u64 {
        self.push_undo(before);
        self.redo.clear();
        self.dirty = true;
        self.edited_since_refresh = true;
        self.serial
    }

    fn push_undo(&mut self, map: MindMap) {
        self.serial += 1;
        let state = Selected { map, selection: self.selection.clone(), focus: self.focus };
        let checkpoint =
            Checkpoint { serial: self.serial, state, dirty: self.dirty, edited: self.edited_since_refresh };
        self.undo.push_back(checkpoint);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.pop_front();
        }
    }

    /// Makes `state` current, returning the current map with its selection.
    fn restore(&mut self, state: Selected) -> Selected {
        let map = self.replace_map(state.map);
        let selection = std::mem::replace(&mut self.selection, state.selection);
        let focus = std::mem::replace(&mut self.focus, state.focus);
        Selected { map, selection, focus }
    }

    /// Makes `map` (an earlier or later copy) the current map and returns the
    /// replaced one. Nodes created from now on get ids no node of the replaced
    /// map had, so a node id held from before never names another node.
    fn replace_map(&mut self, map: MindMap) -> MindMap {
        let bound = self.map.id_bound();
        let replaced = std::mem::replace(&mut self.map, map);
        self.map.reserve_ids(bound);
        replaced
    }

    /// Makes a change that may fail; only a successful one is recorded for undo.
    pub(super) fn try_change<T, E>(&mut self, change: impl FnOnce(&mut MindMap) -> Result<T, E>) -> Result<T, E> {
        let before = self.map.clone();
        let result = change(&mut self.map);
        if result.is_ok() {
            self.record(before);
        }
        result
    }

    /// Drops the undo and redo steps: after a change that also moved files,
    /// which undoing can't take back, so no step may bring back a map that
    /// names the files as they were.
    pub(super) fn forget_undo(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Applies the changes of a background refresh (`apply` tells whether it
    /// made any). It is no undo step of its own: it is applied to the maps
    /// undo and redo go back to as well, so that undoing takes back the
    /// user's own changes and keeps the refresh. Unlike an edit, it doesn't
    /// call for another refresh.
    pub(super) fn apply_refresh(&mut self, mut apply: impl FnMut(&mut MindMap) -> bool) {
        if !apply(&mut self.map) {
            return;
        }
        self.dirty = true;
        // Nodes the refresh creates in one map get ids no other map has.
        let mut bound = self.map.id_bound();
        let maps = self.undo.iter_mut().map(|c| {
            c.dirty = true;
            &mut c.state.map
        });
        for map in maps.chain(self.redo.iter_mut().map(|r| &mut r.map)) {
            map.reserve_ids(bound);
            apply(map);
            bound = bound.max(map.id_bound());
        }
        self.map.reserve_ids(bound);
    }

    /// Takes back the creation of `node`, recorded by the checkpoint `serial`
    /// (a new node whose text dialog was cancelled). Changes recorded since
    /// (a background refresh) are kept; the node is deleted from them instead,
    /// so that no undo step brings it back.
    pub(super) fn cancel_new_node(&mut self, node: NodeId, serial: u64) {
        match self.undo.iter().position(|c| c.serial == serial) {
            // Nothing happened since: back to the map before the node, parent's
            // fold, selection and unsaved mark included.
            Some(i) if i + 1 == self.undo.len() => {
                if let Some(before) = self.undo.pop_back() {
                    self.restore(before.state);
                    self.dirty = before.dirty;
                    self.edited_since_refresh = before.edited;
                }
            }
            Some(i) => {
                self.undo.remove(i);
                for c in self.undo.range_mut(i..) {
                    c.state.map.delete(node);
                }
                for r in &mut self.redo {
                    r.map.delete(node);
                }
                self.map.delete(node);
            }
            None => self.map.delete(node),
        }
    }

    /// Goes back to the map before the last change, with the selection it had.
    pub(super) fn undo(&mut self) {
        if let Some(prev) = self.undo.pop_back() {
            let current = self.restore(prev.state);
            self.redo.push(current);
            self.dirty = true;
        }
    }

    /// Goes forward to the map after the change undone last, with the
    /// selection it had.
    pub(super) fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let current = self.restore(next);
            self.serial += 1;
            let checkpoint = Checkpoint { serial: self.serial, state: current, dirty: true, edited: true };
            self.undo.push_back(checkpoint);
            if self.undo.len() > UNDO_LIMIT {
                self.undo.pop_front();
            }
            self.dirty = true;
        }
    }

    // --- Selection ---

    /// Selects just these nodes, focusing the last one; an empty list changes nothing.
    pub(super) fn select(&mut self, nodes: Vec<NodeId>) {
        if let Some(&last) = nodes.last() {
            self.focus = last;
            self.selection = nodes;
        }
    }

    /// Adds the node to the selection and focuses it (Shift).
    pub(super) fn extend_selection(&mut self, id: NodeId) {
        if !self.selection.contains(&id) {
            self.selection.push(id);
        }
        self.focus = id;
    }

    /// Adds the node to the selection, or takes it out unless it is the only
    /// selected node (Ctrl).
    pub(super) fn toggle_selected(&mut self, id: NodeId) {
        if let Some(pos) = self.selection.iter().position(|&n| n == id) {
            if self.selection.len() > 1 {
                self.selection.remove(pos);
                self.focus = *self.selection.last().unwrap();
            }
        } else {
            self.selection.push(id);
            self.focus = id;
        }
    }

    /// The selection in document order, without nodes under another selected node.
    pub(super) fn sorted_selection(&self) -> Vec<NodeId> {
        pkm::sorted_selection(&self.map, &self.selection)
    }

    /// Drops deleted nodes from the selection. When none is left, the node
    /// nearest to where the focused one was is selected: its next sibling
    /// still there, else its previous one, else its nearest ancestor.
    fn fix_selection(&mut self) {
        let map = &self.map;
        self.selection.retain(|&n| map.exists(n));
        if !map.exists(self.focus) {
            self.focus = match self.selection.last() {
                Some(&n) => n,
                None => self.near(self.focus),
            };
        }
        if self.selection.is_empty() {
            self.selection = vec![self.focus];
        }
        self.shown = self.map.clone();
    }

    /// The node of the map nearest to where `gone` was in the map last laid
    /// out: a sibling still there, else the parent, else the same for the
    /// parent.
    fn near(&self, gone: NodeId) -> NodeId {
        let (map, shown) = (&self.map, &self.shown);
        let mut n = gone;
        while shown.exists(n) {
            let Some(parent) = shown.parent(n) else { break };
            let sibs = shown.children(parent);
            let pos = sibs.iter().position(|&s| s == n).unwrap_or(0);
            let sibling = sibs[pos + 1..].iter().chain(sibs[..pos].iter().rev()).find(|&&s| map.exists(s));
            if let Some(&s) = sibling {
                return s;
            }
            if map.exists(parent) {
                return parent;
            }
            n = parent;
        }
        map.root()
    }

    /// Hands the selection of a node the layout leaves out (hidden by the
    /// filter or a fold) to its nearest shown ancestor.
    fn select_shown(&mut self) {
        let (map, layout) = (&self.map, &self.layout);
        let shown = |n: NodeId| std::iter::once(n).chain(map.ancestors(n)).find(|&a| layout.contains(a));
        let shown = |n: NodeId| shown(n).unwrap_or(map.root());
        let mut selection: Vec<NodeId> = self.selection.iter().map(|&n| shown(n)).collect();
        selection.dedup();
        self.focus = shown(self.focus);
        if !selection.contains(&self.focus) {
            selection.push(self.focus);
        }
        self.selection = selection;
    }

    // --- Layout ---

    /// Lays out the shown part of the map at the zoom, with the margins
    /// around it (left and above, right and below); text sizes are measured
    /// once and kept in `measure_cache`, which holds sizes at this zoom only.
    pub(super) fn relayout(&mut self, margins: (Size, Size), zoom: f32, measure_cache: &mut HashMap<String, Size>) {
        self.fix_selection();
        let visible = self.filter.visible(&self.map);
        let map = &self.map;
        self.layout =
            layout::layout(map, &visible, margins, zoom, &mut |id| view::node_size(measure_cache, map, id, zoom));
        self.select_shown();
    }

    pub(super) fn focus_rect(&self) -> Option<Rectangle> {
        self.layout.get(self.focus).map(|b| b.rect)
    }

    // --- Edits ---

    /// Creates an empty child of the focused node and selects it. Returns the
    /// node and the serial number of the checkpoint before it.
    pub(super) fn new_child(&mut self) -> (NodeId, u64) {
        let serial = self.checkpoint();
        self.map.set_folded(self.focus, false);
        let n = self.map.create_child(self.focus, None);
        self.select(vec![n]);
        (n, serial)
    }

    /// Creates an empty node after the focused one (a child, for the root) and
    /// selects it. Returns the node and the serial number of the checkpoint before it.
    pub(super) fn new_sibling(&mut self) -> (NodeId, u64) {
        let serial = self.checkpoint();
        let focus = self.focus;
        let n = match self.map.parent(focus) {
            Some(p) => {
                let index = self.map.index_in_parent(focus).map(|i| i + 1);
                self.map.create_child(p, index)
            }
            None => self.map.create_child(focus, None),
        };
        self.select(vec![n]);
        (n, serial)
    }

    /// Deletes the selected nodes but the root, and selects a neighbour of the
    /// last one (or else its parent).
    pub(super) fn delete_selected(&mut self) {
        let root = self.map.root();
        let targets: Vec<NodeId> = self.sorted_selection().into_iter().filter(|&n| n != root).collect();
        let Some(&last) = targets.last() else { return };
        let next = self.layout.get(last).and_then(|b| b.parent).map(|p| {
            let sibs = self.map.children(p);
            let pos = sibs.iter().position(|&s| s == last).unwrap_or(0);
            sibs.iter()
                .skip(pos + 1)
                .chain(sibs[..pos].iter().rev())
                .find(|s| !targets.contains(s))
                .copied()
                .unwrap_or(p)
        });
        self.checkpoint();
        for t in targets {
            self.map.delete(t);
        }
        if let Some(n) = next {
            self.select(vec![n]);
        }
    }

    /// Moves the focused node one place up or down among its siblings.
    pub(super) fn move_focus(&mut self, up: bool) {
        let focus = self.focus;
        let Some(parent) = self.map.parent(focus) else { return };
        let index = self.map.index_in_parent(focus).unwrap_or(0);
        let new_index = if up { index.checked_sub(1) } else { Some(index + 1) };
        if let Some(i) = new_index.filter(|&i| i < self.map.children(parent).len()) {
            self.checkpoint();
            self.map.move_node(focus, parent, Some(i));
        }
    }

    /// Folds the selected nodes with children, or unfolds them all when the
    /// first one is folded.
    pub(super) fn toggle_fold_selected(&mut self) {
        let targets: Vec<NodeId> = self.sorted_selection().into_iter().filter(|&n| self.map.has_children(n)).collect();
        let Some(&first) = targets.first() else { return };
        self.checkpoint();
        let fold = !self.map.is_folded(first);
        for n in targets {
            self.map.set_folded(n, fold);
        }
    }

    pub(super) fn toggle_folded(&mut self, id: NodeId) {
        let folded = self.map.is_folded(id);
        self.checkpoint();
        self.map.set_folded(id, !folded);
    }

    /// Fold One Level (F3) or Unfold One Level (F4) on every selected node.
    pub(super) fn fold_one_level(&mut self, fold: bool) {
        let selection = self.sorted_selection();
        let fold_one_level = if fold { commands::fold_one_level } else { commands::unfold_one_level };
        self.checkpoint();
        for n in selection {
            fold_one_level(&mut self.map, n);
        }
    }

    /// Unfolds the way to the node and selects it.
    pub(super) fn reveal(&mut self, id: NodeId) {
        self.map.unfold_ancestors(id);
        self.select(vec![id]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Doc {
        Doc::new(MindMap::new("root"), 1)
    }

    #[test]
    fn node_numbers_are_not_reused_after_undo() {
        let mut doc = doc();
        let root = doc.map.root();
        let (a, _) = doc.new_child();
        doc.undo();
        doc.select(vec![root]);
        let (b, _) = doc.new_child();
        assert_ne!(a, b);
        doc.undo();
        doc.redo();
        doc.select(vec![root]);
        let (c, _) = doc.new_child();
        assert!(c != a && c != b);
    }

    #[test]
    fn edits_call_for_a_refresh_but_the_refresh_itself_does_not() {
        let mut doc = doc();
        let root = doc.map.root();
        doc.apply_refresh(|map| {
            map.add_child(root, "from refresh");
            true
        });
        assert!(!doc.edited_since_refresh);
        assert!(doc.dirty);
        doc.new_child();
        assert!(doc.edited_since_refresh);
    }

    #[test]
    fn undo_takes_back_the_users_change_and_keeps_the_refresh() {
        let mut doc = doc();
        let root = doc.map.root();
        let a = doc.map.add_child(root, "a");
        doc.select(vec![a]);
        let (n, _) = doc.new_child();
        doc.map.set_text(n, "pasted");
        // The refresh renames nothing but adds a node, to every map.
        doc.apply_refresh(|map| {
            map.add_child(root, "from refresh");
            true
        });
        doc.undo();
        assert_eq!(doc.map.child_texts(root), vec!["a", "from refresh"]);
        assert!(doc.map.child_texts(a).is_empty());
        // The selection is the one before the change.
        assert_eq!((doc.focus, doc.selection.clone()), (a, vec![a]));
        doc.redo();
        assert_eq!(doc.map.child_texts(a), vec!["pasted"]);
        assert_eq!(doc.map.child_texts(root), vec!["a", "from refresh"]);
        assert_eq!(doc.focus, n);
        // Nothing older to undo; ids created in the maps are all different.
        doc.undo();
        doc.undo();
        assert_eq!(doc.map.child_texts(root), vec!["a", "from refresh"]);
        let refreshed = doc.map.children(root)[1];
        doc.redo();
        assert_ne!(doc.map.children(root)[1], refreshed);
    }

    #[test]
    fn a_selected_node_that_goes_hands_the_selection_to_a_neighbour() {
        let mut doc = doc();
        let root = doc.map.root();
        let a = doc.map.add_child(root, "a");
        let b = doc.map.add_child(root, "b");
        let b1 = doc.map.add_child(b, "b1");
        let c = doc.map.add_child(root, "c");
        let relayout = |doc: &mut Doc| doc.relayout((Size::ZERO, Size::ZERO), 1.0, &mut HashMap::new());
        doc.select(vec![b]);
        relayout(&mut doc);
        doc.map.delete(b);
        relayout(&mut doc);
        assert_eq!(doc.focus, c);
        doc.map.delete(c);
        relayout(&mut doc);
        assert_eq!(doc.focus, a);
        // A deleted subtree: the parent's neighbour, or the ancestor.
        let d = doc.map.add_child(a, "d");
        let d1 = doc.map.add_child(d, "d1");
        doc.select(vec![d1]);
        relayout(&mut doc);
        doc.map.delete(d);
        relayout(&mut doc);
        assert_eq!(doc.focus, a);
        assert!(!doc.map.exists(b1));
    }

    #[test]
    fn cancelling_a_new_node_right_away_restores_the_map() {
        let mut doc = doc();
        let root = doc.map.root();
        let a = doc.map.add_child(root, "a");
        doc.map.add_child(a, "a1");
        doc.map.set_folded(a, true);
        doc.select(vec![a]);
        let before = doc.map.to_xml();
        let (n, serial) = doc.new_child();
        doc.cancel_new_node(n, serial);
        // The parent is folded again, and no undo step is left behind.
        assert_eq!(doc.map.to_xml(), before);
        doc.undo();
        assert_eq!(doc.map.to_xml(), before);
    }

    #[test]
    fn cancelling_a_new_node_keeps_changes_made_meanwhile() {
        let mut doc = doc();
        let root = doc.map.root();
        let before_tab = doc.map.to_xml();
        let (n, serial) = doc.new_child();
        // A Next Steps refresh lands while the text dialog is open.
        let mut refreshed = None;
        doc.apply_refresh(|map| {
            refreshed = Some(map.add_child(root, "from refresh"));
            true
        });
        let refreshed = refreshed.unwrap();
        doc.cancel_new_node(n, serial);
        assert!(!doc.map.exists(n));
        assert_eq!(doc.map.child_texts(root), vec!["from refresh"]);
        // No undo step brings the new node back or takes the refresh away.
        doc.undo();
        assert!(doc.map.exists(refreshed));
        assert!(!doc.map.exists(n));
        assert_ne!(doc.map.to_xml(), before_tab);
    }
}
