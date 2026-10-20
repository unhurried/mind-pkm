//! An open map with its view state: the selection, the filter, the layout and
//! the undo history, and the edits that only concern the map.

use super::layout::{self, MapLayout};
use super::view;
use crate::mindmap::{MindMap, NodeId};
use crate::pkm::{self, commands, filter::Filter};
use iced::{Rectangle, Size};
use std::collections::{HashMap, VecDeque};

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
    undo: VecDeque<Checkpoint>,
    redo: Vec<MindMap>,
    /// The serial number of the last checkpoint.
    serial: u64,
}

/// The map as it was before a change, with the serial number identifying it.
struct Checkpoint {
    serial: u64,
    map: MindMap,
}

impl Doc {
    pub(super) fn new(map: MindMap, generation: u64) -> Self {
        let root = map.root();
        Doc {
            generation,
            map,
            selection: vec![root],
            focus: root,
            filter: Filter::HideArchive,
            layout: MapLayout::default(),
            dirty: false,
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

    /// Records `before` as the map preceding a change just made. Returns the
    /// checkpoint's serial number.
    pub(super) fn record(&mut self, before: MindMap) -> u64 {
        self.push_undo(before);
        self.redo.clear();
        self.dirty = true;
        self.serial
    }

    fn push_undo(&mut self, map: MindMap) {
        self.serial += 1;
        self.undo.push_back(Checkpoint { serial: self.serial, map });
        if self.undo.len() > UNDO_LIMIT {
            self.undo.pop_front();
        }
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

    /// Takes back the creation of `node`, recorded by the checkpoint `serial`
    /// (a new node whose text dialog was cancelled). Changes recorded since
    /// (a background refresh) are kept; the node is deleted from them instead,
    /// so that no undo step brings it back.
    pub(super) fn cancel_new_node(&mut self, node: NodeId, serial: u64) {
        match self.undo.iter().position(|c| c.serial == serial) {
            // Nothing happened since: back to the map before the node, parent's fold included.
            Some(i) if i + 1 == self.undo.len() => {
                let before = self.undo.pop_back().map(|c| c.map);
                if let Some(before) = before {
                    self.replace_map(before);
                }
            }
            Some(i) => {
                self.undo.remove(i);
                for c in self.undo.range_mut(i..) {
                    c.map.delete(node);
                }
                for m in &mut self.redo {
                    m.delete(node);
                }
                self.map.delete(node);
            }
            None => self.map.delete(node),
        }
    }

    pub(super) fn undo(&mut self) {
        if let Some(prev) = self.undo.pop_back() {
            let current = self.replace_map(prev.map);
            self.redo.push(current);
            self.dirty = true;
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let current = self.replace_map(next);
            self.push_undo(current);
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

    /// Drops deleted nodes from the selection; falls back to the root.
    fn fix_selection(&mut self) {
        let map = &self.map;
        self.selection.retain(|&n| map.exists(n));
        if !map.exists(self.focus) {
            self.focus = self.selection.last().copied().unwrap_or(map.root());
        }
        if self.selection.is_empty() {
            self.selection = vec![self.focus];
        }
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

    /// Lays out the shown part of the map at the zoom, with `margin` around it;
    /// text sizes are measured once and kept in `measure_cache`, which holds
    /// sizes at this zoom only.
    pub(super) fn relayout(&mut self, margin: Size, zoom: f32, measure_cache: &mut HashMap<String, Size>) {
        self.fix_selection();
        let visible = self.filter.visible(&self.map);
        let map = &self.map;
        self.layout =
            layout::layout(map, &visible, margin, zoom, &mut |id| view::node_size(measure_cache, map, id, zoom));
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
        self.checkpoint();
        for n in selection {
            if fold {
                commands::fold_one_level(&mut self.map, n);
            } else {
                commands::unfold_one_level(&mut self.map, n);
            }
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
        let before_refresh = doc.map.clone();
        let refreshed = doc.map.add_child(root, "from refresh");
        doc.record(before_refresh);
        doc.cancel_new_node(n, serial);
        assert!(!doc.map.exists(n));
        assert_eq!(doc.map.child_texts(root), vec!["from refresh"]);
        // Undo goes back to before the refresh, without the new node, and
        // nothing older brings it back.
        doc.undo();
        assert!(!doc.map.exists(refreshed));
        assert_eq!(doc.map.to_xml(), before_tab);
        doc.undo();
        assert_eq!(doc.map.to_xml(), before_tab);
    }
}
