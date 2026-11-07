//! An open map with its view state: the selection, the filter, the layout and
//! the undo history, and the edits that only concern the map.
//!
//! The map and its state change only through `Doc`: every edit is recorded
//! for undo (`change`, `try_change`, `change_recorded_if`...), and the few
//! changes that are not say why (`unfold_to_navigate`, `reveal`,
//! `apply_refresh`, `set_new_node_text`).

use super::drag::{DropPlace, DropTarget};
use super::layout::{self, MapLayout};
use super::view;
use crate::mindmap::{MindMap, NodeId};
use crate::pkm::{self, commands, filter::Filter};
use iced::{Rectangle, Size};
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::time::{Duration, Instant};

const UNDO_LIMIT: usize = 100;

pub struct Doc {
    /// Distinguishes this document from a later one in background results.
    generation: u64,
    map: MindMap,
    /// The selected nodes, in the order they were selected.
    selection: Vec<NodeId>,
    /// The node single-node commands act on; always among the selection.
    focus: NodeId,
    filter: Filter,
    layout: MapLayout,
    /// Changed since last saved.
    dirty: bool,
    /// When the last Next Steps / search index refresh started, and whether
    /// the map has been edited since.
    last_refresh: Instant,
    edited_since_refresh: bool,
    undo: VecDeque<Checkpoint>,
    redo: Vec<Selected>,
    /// The serial number of the last checkpoint.
    serial: u64,
    /// The map as last laid out: where a selected node that is gone was.
    shown: MindMap,
    /// What the layout was computed from.
    laid_out: Option<LayoutInputs>,
    /// The nodes the filter shows, for a version of the map and the filter.
    visible: Option<(u64, Filter, HashSet<NodeId>)>,
    /// The anchor of the last Shift+click range, with the selection it made:
    /// the next Shift+click starts from it while the selection is unchanged.
    range: Option<(NodeId, Vec<NodeId>)>,
}

/// What a layout depends on: the map (its version), the filter, the margin,
/// the zoom and the text being edited.
#[derive(Debug, PartialEq)]
struct LayoutInputs {
    map: u64,
    filter: Filter,
    margin: Size,
    zoom: f32,
    draft: Option<Draft>,
}

/// The text typed so far into a node being edited: the node is laid out with
/// it, and shown even if the filter hides it.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub id: NodeId,
    pub text: String,
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
            laid_out: None,
            visible: None,
            range: None,
        }
    }

    // --- State ---

    /// Distinguishes this document from a later one in background results.
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn map(&self) -> &MindMap {
        &self.map
    }

    /// The selected nodes, in the order they were selected.
    pub(super) fn selection(&self) -> &[NodeId] {
        &self.selection
    }

    /// The node single-node commands act on; always among the selection.
    pub(super) fn focus(&self) -> NodeId {
        self.focus
    }

    pub(super) fn filter(&self) -> &Filter {
        &self.filter
    }

    pub(super) fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
    }

    pub(super) fn layout(&self) -> &MapLayout {
        &self.layout
    }

    /// Changed since last saved.
    pub(super) fn dirty(&self) -> bool {
        self.dirty
    }

    /// Whether the map has been edited since the last Next Steps / search
    /// index refresh started.
    #[cfg(test)]
    pub(super) fn edited_since_refresh(&self) -> bool {
        self.edited_since_refresh
    }

    /// Whether a refresh is due: the map has been edited since the last one,
    /// which started at least `interval` ago.
    pub(super) fn refresh_due(&self, interval: Duration) -> bool {
        self.edited_since_refresh && self.last_refresh.elapsed() >= interval
    }

    /// A Next Steps / search index refresh starts.
    pub(super) fn refresh_started(&mut self) {
        self.last_refresh = Instant::now();
        self.edited_since_refresh = false;
    }

    /// Saves the map to `path`; it has no unsaved changes then.
    pub(super) fn save(&mut self, path: &Path) -> Result<(), String> {
        self.map.save(path)?;
        self.dirty = false;
        Ok(())
    }

    // --- Undo ---

    /// Records the current map for undo; call before every change. Returns the
    /// checkpoint's serial number.
    fn checkpoint(&mut self) -> u64 {
        self.record(self.map.clone())
    }

    /// Records `before` as the map preceding a change just made (with the
    /// selection, which the change hasn't touched yet). Returns the
    /// checkpoint's serial number.
    fn record(&mut self, before: MindMap) -> u64 {
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

    /// Makes a change, recorded for undo.
    pub(super) fn change<T>(&mut self, change: impl FnOnce(&mut MindMap) -> T) -> T {
        self.checkpoint();
        change(&mut self.map)
    }

    /// Makes a change, recorded for undo only when `recorded` says so (given
    /// the map before it, the map after it and the change's result): a change
    /// not recorded stays as it is.
    pub(super) fn change_recorded_if<T>(
        &mut self,
        change: impl FnOnce(&mut MindMap) -> T,
        recorded: impl FnOnce(&MindMap, &MindMap, &T) -> bool,
    ) -> T {
        let before = self.map.clone();
        let result = change(&mut self.map);
        if recorded(&before, &self.map, &result) {
            self.record(before);
        }
        result
    }

    /// Makes a change, recorded for undo when `kept` says so (given the map
    /// after it); else the map is put back as it was.
    pub(super) fn change_or_revert(&mut self, change: impl FnOnce(&mut MindMap), kept: impl FnOnce(&MindMap) -> bool) {
        let before = self.map.clone();
        change(&mut self.map);
        if kept(&self.map) {
            self.record(before);
        } else {
            self.map = before;
        }
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
    /// so that no undo step brings it back. The nodes selected before the new
    /// node was created are selected again (those still there; else its parent).
    pub(super) fn cancel_new_node(&mut self, node: NodeId, serial: u64) {
        let parent = self.map.exists(node).then(|| self.map.parent(node)).flatten();
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
                let before = self.undo.remove(i).map(|c| c.state);
                for c in self.undo.range_mut(i..) {
                    c.state.map.delete(node);
                }
                for r in &mut self.redo {
                    r.map.delete(node);
                }
                self.map.delete(node);
                match before {
                    Some(b) => self.select_again(b.selection, b.focus, parent),
                    None => self.select_again(Vec::new(), node, parent),
                }
            }
            None => {
                self.map.delete(node);
                self.select_again(Vec::new(), node, parent);
            }
        }
    }

    /// Selects `selection` again with its `focus`, leaving out the nodes that
    /// are gone; when none is left, `fallback` (if it is still there).
    fn select_again(&mut self, selection: Vec<NodeId>, focus: NodeId, fallback: Option<NodeId>) {
        let mut kept: Vec<NodeId> = selection.into_iter().filter(|&n| self.map.exists(n)).collect();
        if kept.is_empty() {
            kept.extend(fallback.filter(|&p| self.map.exists(p)));
        }
        let Some(&last) = kept.last() else { return };
        self.focus = if kept.contains(&focus) { focus } else { last };
        self.selection = kept;
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

    /// Selects the nodes from the anchor to `id`, both included, and focuses
    /// `id` (Shift+click): the siblings between them if they are siblings,
    /// else the shown nodes between them in document order. The anchor is the
    /// one of the last range while the selection is still that range, else
    /// the focused node.
    pub(super) fn select_range(&mut self, id: NodeId) {
        let (map, layout) = (&self.map, &self.layout);
        let usable = |n: NodeId| map.exists(n) && layout.contains(n);
        let anchor = match &self.range {
            Some((a, range)) if *range == self.selection && usable(*a) => *a,
            _ => self.focus,
        };
        if !usable(anchor) || !usable(id) {
            self.extend_selection(id);
            return;
        }
        let parent = map.parent(anchor);
        let order = match parent {
            Some(p) if parent == map.parent(id) => map.children(p),
            _ => map.preorder(map.root()),
        };
        let order: Vec<NodeId> = order.into_iter().filter(|&n| layout.contains(n)).collect();
        let (Some(from), Some(to)) = (order.iter().position(|&n| n == anchor), order.iter().position(|&n| n == id))
        else {
            self.extend_selection(id);
            return;
        };
        let mut nodes: Vec<NodeId> = order[from.min(to)..=from.max(to)].to_vec();
        if to < from {
            // The clicked node last, so that it is the focused one.
            nodes.reverse();
        }
        self.select(nodes);
        self.range = Some((anchor, self.selection.clone()));
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

    /// Lays out the shown part of the map at the zoom, with `margin` around
    /// it, unless nothing the layout depends on changed since the last time
    /// (the map, the filter, the margin, the zoom); text sizes are measured
    /// once and kept in `measure_cache`, at 100% (`view::text_at_zoom`).
    /// What the filter shows is kept for the map's version, and when the map
    /// and the zoom are the same as last time (only the text being edited,
    /// the filter or the margin changed), the nodes' sizes are those they
    /// were laid out with but for the node being edited.
    /// Either way the selection is kept to existing, shown nodes. Returns
    /// whether the map was laid out anew.
    pub(super) fn relayout(
        &mut self,
        margin: Size,
        zoom: f32,
        measure_cache: &mut view::MeasureCache,
        draft: Option<Draft>,
    ) -> bool {
        self.fix_selection();
        let inputs = LayoutInputs { map: self.map.version(), filter: self.filter.clone(), margin, zoom, draft };
        let anew = self.laid_out.as_ref() != Some(&inputs);
        if anew {
            self.shown = self.map.clone();
            if !self.visible.as_ref().is_some_and(|(v, f, _)| *v == inputs.map && *f == inputs.filter) {
                self.visible = Some((inputs.map, inputs.filter.clone(), self.filter.visible(&self.map)));
            }
            let map = &self.map;
            let mut visible = Cow::Borrowed(&self.visible.as_ref().expect("just made").2);
            let draft = inputs.draft.as_ref().filter(|d| map.exists(d.id));
            if let Some(d) = draft {
                // The node being edited is shown, under its ancestors.
                for n in std::iter::once(d.id).chain(map.ancestors(d.id)) {
                    if !visible.contains(&n) {
                        visible.to_mut().insert(n);
                    }
                }
            }
            // The sizes the last layout gave, if they still hold.
            let previous = self.laid_out.as_ref().filter(|p| p.map == inputs.map && p.zoom == zoom);
            let edited = [previous.and_then(|p| p.draft.as_ref()).map(|d| d.id), draft.map(|d| d.id)];
            let last = &self.layout;
            let layout = layout::layout(map, &visible, margin, zoom, &mut |id| {
                let kept = previous.filter(|_| !edited.contains(&Some(id))).and_then(|_| last.get(id));
                match kept {
                    Some(b) => b.rect.size(),
                    None => view::node_size(measure_cache, map, id, zoom, draft),
                }
            });
            self.layout = layout;
            self.laid_out = Some(inputs);
        }
        self.select_shown();
        anew
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

    /// Makes the focused node the last child of its previous sibling (the
    /// node above it), unfolding that node. Nothing happens for the root or a
    /// first child.
    pub(super) fn indent_focus(&mut self) {
        let focus = self.focus;
        let above = self.map.parent(focus).and_then(|parent| {
            let index = self.map.index_in_parent(focus)?.checked_sub(1)?;
            self.map.children(parent).get(index).copied()
        });
        if let Some(node) = above {
            self.move_nodes(&[focus], DropTarget { node, place: DropPlace::Child });
        }
    }

    /// Makes the focused node the next sibling of its parent. Nothing happens
    /// for the root or a child of the root.
    pub(super) fn outdent_focus(&mut self) {
        let parent = self.map.parent(self.focus).filter(|&p| self.map.parent(p).is_some());
        if let Some(node) = parent {
            self.move_nodes(&[self.focus], DropTarget { node, place: DropPlace::After });
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

    /// Names a node just created (`new_child`, `new_sibling`): part of the
    /// undo step of its creation.
    pub(super) fn set_new_node_text(&mut self, id: NodeId, text: &str) {
        self.map.set_text(id, text);
    }

    /// Unfolds a node the user goes into (Right on a folded node). Going
    /// there is no edit: no undo step, nothing marked as changed.
    pub(super) fn unfold_to_navigate(&mut self, id: NodeId) {
        self.map.set_folded(id, false);
    }

    /// Makes `id`, one of the selected nodes, the one single-node commands act on.
    pub(super) fn set_focus(&mut self, id: NodeId) {
        if self.selection.contains(&id) {
            self.focus = id;
        }
    }

    /// Unfolds the way to the node and selects it. Like going into a folded
    /// node, this is no edit: no undo step, nothing marked as changed.
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

    /// While a node's text is edited, only that node is measured again: the
    /// others keep the sizes they were laid out with, until the map or the
    /// zoom changes.
    #[test]
    fn only_the_node_being_edited_is_measured_again_while_typing() {
        let mut doc = doc();
        let root = doc.map.root();
        let a = doc.map.add_child(root, "a");
        let b = doc.map.add_child(root, "bb");
        let mut cache = view::MeasureCache::default();
        let draft = |text: &str| Some(Draft { id: a, text: text.into() });
        let width = |doc: &Doc, id| doc.layout.get(id).unwrap().rect.width;
        assert!(doc.relayout(Size::ZERO, 1.0, &mut cache, None));
        let (a_width, b_width) = (width(&doc, a), width(&doc, b));
        assert!(doc.relayout(Size::ZERO, 1.0, &mut cache, draft("a")));
        let typed = width(&doc, a);
        assert!(doc.relayout(Size::ZERO, 1.0, &mut cache, draft("a much longer text")));
        assert!(width(&doc, a) > typed);
        assert_eq!(width(&doc, b), b_width);
        // Cancelled: the node's own text again.
        assert!(doc.relayout(Size::ZERO, 1.0, &mut cache, None));
        assert_eq!((width(&doc, a), width(&doc, b)), (a_width, b_width));
        // Another node's text changed meanwhile (a new version of the map): measured again.
        doc.map.set_text(b, "b with a longer text");
        assert!(doc.relayout(Size::ZERO, 1.0, &mut cache, draft("a")));
        assert_eq!(width(&doc, b), box_width(&doc.map, b));
        // Another zoom: every node at it.
        assert!(doc.relayout(Size::ZERO, 2.0, &mut cache, draft("a")));
        let at_2 = view::node_size(&mut cache, &doc.map, b, 2.0, None).width;
        assert_eq!(width(&doc, b), at_2);
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
    fn the_map_is_laid_out_again_only_when_its_layout_may_change() {
        let mut doc = doc();
        let root = doc.map.root();
        let a = doc.map.add_child(root, "a");
        let a1 = doc.map.add_child(a, "a1");
        let mut cache = view::MeasureCache::default();
        let mut relayout = |doc: &mut Doc, zoom: f32| doc.relayout(Size::ZERO, zoom, &mut cache, None);
        assert!(relayout(&mut doc, 1.0));
        assert!(!relayout(&mut doc, 1.0));
        // Selecting changes nothing laid out.
        doc.select(vec![a1]);
        assert!(!relayout(&mut doc, 1.0));
        // The map, the filter and the zoom do.
        doc.change(|map| map.set_text(a1, "a1 renamed"));
        assert!(relayout(&mut doc, 1.0));
        assert_eq!(doc.layout.get(a1).unwrap().rect.width, box_width(&doc.map, a1));
        doc.undo();
        assert!(relayout(&mut doc, 1.0));
        doc.set_filter(Filter::All);
        assert!(relayout(&mut doc, 1.0));
        assert!(relayout(&mut doc, 2.0));
        assert!(!relayout(&mut doc, 2.0));
        // A fold hiding the selected node: laid out anew, the selection to the folded node.
        doc.toggle_folded(a);
        assert!(relayout(&mut doc, 2.0));
        assert!(!doc.layout.contains(a1));
        assert_eq!(doc.focus, a);
    }

    /// The width of a node's box at 100%, as laid out.
    fn box_width(map: &MindMap, id: NodeId) -> f32 {
        view::node_size(&mut view::MeasureCache::default(), map, id, 1.0, None).width
    }

    #[test]
    fn a_selected_node_that_goes_hands_the_selection_to_a_neighbour() {
        let mut doc = doc();
        let root = doc.map.root();
        let a = doc.map.add_child(root, "a");
        let b = doc.map.add_child(root, "b");
        let b1 = doc.map.add_child(b, "b1");
        let c = doc.map.add_child(root, "c");
        let relayout = |doc: &mut Doc| doc.relayout(Size::ZERO, 1.0, &mut view::MeasureCache::default(), None);
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
