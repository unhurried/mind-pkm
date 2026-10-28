//! Moving nodes by drag and drop.
//!
//! A press on a selected node arms a drag of the selection; it starts once the
//! pointer moved a little, and the node under the pointer gives the drop
//! position. Pointer positions are in map coordinates, reported by a mouse
//! area inside the map's scrollable while a drag is armed.

use super::doc::Doc;
use super::layout::MapLayout;
use super::{App, Message};
use crate::mindmap::{MindMap, NodeId};
use iced::Point;

/// How far the pointer has to move before a press becomes a drag.
const DRAG_THRESHOLD: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPlace {
    /// The previous sibling of the target.
    Before,
    /// The next sibling of the target.
    After,
    /// The last child of the target.
    Child,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropTarget {
    pub node: NodeId,
    pub place: DropPlace,
}

#[derive(Debug, Clone)]
pub struct NodeDrag {
    /// The dragged nodes, in map order.
    nodes: Vec<NodeId>,
    /// Where the pointer was first seen after the press; None before that.
    origin: Option<Point>,
    /// The pointer moved past the threshold.
    dragging: bool,
    /// Where the nodes would go if dropped now.
    pub target: Option<DropTarget>,
    /// The node to select alone if the button is released without a drag.
    click: Option<NodeId>,
}

/// The drop position for the pointer at `p`: the node box under it, by its top
/// quarter (before), bottom quarter (after) or the rest (child). The root only
/// takes children; the dragged nodes and their descendants take nothing.
pub fn drop_target(map: &MindMap, layout: &MapLayout, dragged: &[NodeId], p: Point) -> Option<DropTarget> {
    let b = layout.boxes.iter().find(|b| b.rect.contains(p))?;
    let node = b.id;
    if dragged.iter().any(|&d| d == node || map.is_ancestor(d, node)) {
        return None;
    }
    let r = b.rect;
    let place = if node == map.root() {
        DropPlace::Child
    } else if p.y < r.y + r.height / 4.0 {
        DropPlace::Before
    } else if p.y > r.y + r.height * 3.0 / 4.0 {
        DropPlace::After
    } else {
        DropPlace::Child
    };
    Some(DropTarget { node, place })
}

impl Doc {
    /// Moves the nodes (in the given order) to the drop target, as one undo
    /// step; a move that leaves them all where they were is not recorded.
    pub(super) fn move_nodes(&mut self, nodes: &[NodeId], target: DropTarget) {
        let map = &self.map;
        let nodes: Vec<NodeId> = nodes.iter().copied().filter(|&n| map.exists(n) && n != map.root()).collect();
        if nodes.is_empty()
            || !map.exists(target.node)
            || nodes.iter().any(|&n| n == target.node || map.is_ancestor(n, target.node))
        {
            return;
        }
        let parent = match target.place {
            DropPlace::Child => target.node,
            DropPlace::Before | DropPlace::After => match map.parent(target.node) {
                Some(p) => p,
                None => return,
            },
        };
        let places =
            |map: &MindMap| -> Vec<_> { nodes.iter().map(|&n| (map.parent(n), map.index_in_parent(n))).collect() };
        let old_places = places(map);
        let unfolds = target.place == DropPlace::Child && map.is_folded(target.node);
        let before = self.map.clone();
        let map = &mut self.map;
        if unfolds {
            map.set_folded(target.node, false);
        }
        // The position of `anchor` among the parent's children once `n` is taken out.
        let index_without = |map: &MindMap, anchor: NodeId, n: NodeId| {
            map.children(parent).into_iter().filter(|&c| c != n).position(|c| c == anchor)
        };
        let mut anchor = target.node;
        for &n in &nodes {
            let index = match target.place {
                DropPlace::Child => None,
                DropPlace::Before => index_without(map, target.node, n),
                DropPlace::After => index_without(map, anchor, n).map(|i| i + 1),
            };
            map.move_node(n, parent, index);
            anchor = n;
        }
        if unfolds || places(&self.map) != old_places {
            self.record(before);
        } else {
            self.map = before;
        }
    }
}

impl App {
    /// After a press on a node: arms a drag of the selection if the node is
    /// (still) selected and not the root. `click` is selected alone if the
    /// button is released without a drag.
    pub(super) fn drag_arm(&mut self, id: NodeId, click: Option<NodeId>) {
        self.drag = None;
        let Some(doc) = &self.doc else { return };
        if id == doc.map.root() || !doc.selection.contains(&id) {
            return;
        }
        let root = doc.map.root();
        let nodes: Vec<NodeId> = doc.sorted_selection().into_iter().filter(|&n| n != root).collect();
        if !nodes.is_empty() {
            self.drag = Some(NodeDrag { nodes, origin: None, dragging: false, target: None, click });
        }
    }

    pub(super) fn drag_move(&mut self, p: Point) {
        let (Some(drag), Some(doc)) = (&mut self.drag, &self.doc) else { return };
        let origin = *drag.origin.get_or_insert(p);
        if !drag.dragging && origin.distance(p) > DRAG_THRESHOLD {
            drag.dragging = true;
        }
        if drag.dragging {
            drag.target = drop_target(&doc.map, &doc.layout, &drag.nodes, p);
        }
    }

    /// The button was released: drops the nodes at the shown position.
    pub(super) fn drag_end(&mut self) {
        let (Some(drag), Some(doc)) = (self.drag.take(), &mut self.doc) else { return };
        if !drag.dragging {
            if let Some(id) = drag.click.filter(|&id| doc.map.exists(id)) {
                doc.select(vec![id]);
            }
        } else if let Some(target) = drag.target {
            doc.move_nodes(&drag.nodes, target);
            self.warn_if_under_page(&drag.nodes);
        }
    }

    /// The drop position to show, while dragging.
    pub(super) fn drop_indicator(&self) -> Option<DropTarget> {
        self.drag.as_ref().filter(|d| d.dragging).and_then(|d| d.target)
    }

    /// Esc: cancels a drag in progress (true), if there is one.
    pub(super) fn drag_cancel(&mut self) -> bool {
        self.drag.take().is_some_and(|d| d.dragging)
    }
}

/// The messages that follow the pointer while a drag is armed.
pub fn drag_events() -> iced::Subscription<Message> {
    iced::event::listen_with(|event, _status, _window| match event {
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => Some(Message::NodeDragEnd),
        iced::Event::Mouse(iced::mouse::Event::CursorLeft) => Some(Message::NodeDragCancel),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::layout::layout;
    use super::*;
    use iced::Size;
    use std::collections::HashSet;

    /// root > a (a1, a2), b, c
    fn sample() -> (MindMap, [NodeId; 6]) {
        let mut map = MindMap::new("root");
        let r = map.root();
        let a = map.add_child(r, "a");
        let a1 = map.add_child(a, "a1");
        let a2 = map.add_child(a, "a2");
        let b = map.add_child(r, "b");
        let c = map.add_child(r, "c");
        (map, [r, a, a1, a2, b, c])
    }

    fn texts(map: &MindMap, id: NodeId) -> Vec<String> {
        map.children(id).into_iter().map(|c| map.text(c)).collect()
    }

    #[test]
    fn drop_position_by_part_of_the_target() {
        let (map, [r, a, a1, _, b, _]) = sample();
        let visible: HashSet<NodeId> = map.preorder(r).into_iter().collect();
        let l = layout(&map, &visible, (Size::ZERO, Size::ZERO), 1.0, &mut |_| Size::new(40.0, 20.0));
        let rb = l.get(b).unwrap().rect;
        let at = |y: f32| Point::new(rb.x + 5.0, rb.y + y);
        let target = |p| drop_target(&map, &l, &[a], p).map(|t| t.place);
        assert_eq!(target(at(2.0)), Some(DropPlace::Before));
        assert_eq!(target(at(10.0)), Some(DropPlace::Child));
        assert_eq!(target(at(18.0)), Some(DropPlace::After));
        // The root only takes children.
        let rr = l.get(r).unwrap().rect;
        assert_eq!(
            drop_target(&map, &l, &[a], Point::new(rr.x + 1.0, rr.y + 1.0)).map(|t| t.place),
            Some(DropPlace::Child)
        );
        // Not onto the dragged nodes or their descendants, nor empty space.
        let ra1 = l.get(a1).unwrap().rect;
        assert_eq!(drop_target(&map, &l, &[a], ra1.center()), None);
        assert_eq!(drop_target(&map, &l, &[a], l.get(a).unwrap().rect.center()), None);
        assert_eq!(drop_target(&map, &l, &[a], Point::new(-10.0, -10.0)), None);
    }

    #[test]
    fn moves_nodes_to_the_target() {
        let (map, [r, a, a1, a2, b, c]) = sample();
        let mut doc = Doc::new(map, 0);
        doc.move_nodes(&[a1, b], DropTarget { node: c, place: DropPlace::Child });
        assert_eq!(texts(&doc.map, c), ["a1", "b"]);
        assert_eq!(texts(&doc.map, r), ["a", "c"]);
        assert!(doc.dirty);
        doc.undo();
        assert_eq!(texts(&doc.map, r), ["a", "b", "c"]);
        assert_eq!(texts(&doc.map, a), ["a1", "a2"]);

        // Before / after, also when the moved node comes from in front of the target.
        doc.move_nodes(&[a], DropTarget { node: c, place: DropPlace::Before });
        assert_eq!(texts(&doc.map, r), ["b", "a", "c"]);
        doc.move_nodes(&[b, a2], DropTarget { node: c, place: DropPlace::After });
        assert_eq!(texts(&doc.map, r), ["a", "c", "b", "a2"]);
        assert_eq!(texts(&doc.map, a), ["a1"]);
    }

    #[test]
    fn invalid_or_noop_moves_change_nothing() {
        let (map, [r, a, a1, _, b, c]) = sample();
        let mut doc = Doc::new(map, 0);
        doc.move_nodes(&[a], DropTarget { node: a1, place: DropPlace::Child });
        doc.move_nodes(&[r], DropTarget { node: b, place: DropPlace::Child });
        doc.move_nodes(&[b], DropTarget { node: c, place: DropPlace::Before });
        doc.move_nodes(&[b], DropTarget { node: a, place: DropPlace::After });
        assert!(!doc.dirty);
        assert_eq!(texts(&doc.map, r), ["a", "b", "c"]);
    }

    #[test]
    fn child_drop_unfolds_the_target() {
        let (mut map, [_, a, _, _, b, _]) = sample();
        map.set_folded(a, true);
        let mut doc = Doc::new(map, 0);
        doc.move_nodes(&[b], DropTarget { node: a, place: DropPlace::Child });
        assert!(!doc.map.is_folded(a));
        assert_eq!(texts(&doc.map, a), ["a1", "a2", "b"]);
    }
}
