//! Mind map layout: every subtree grows to the right of its parent, the
//! children stacked vertically and centered on it (a single child a little
//! higher, so that it doesn't continue its parent's underline). All of the
//! root's children are on the right, whatever side Freeplane's POSITION
//! attribute names.

use crate::mindmap::{MindMap, NodeId};
use iced::{Point, Rectangle, Size};
use std::collections::{HashMap, HashSet};

/// Horizontal gap between a node and its children (at 100% zoom).
pub const H_GAP: f32 = 36.0;
/// Vertical gap between sibling subtrees (at 100% zoom).
pub const V_GAP: f32 = 2.0;
/// How much higher than its parent a single child is placed (at 100% zoom).
pub const SINGLE_CHILD_RISE: f32 = 8.0;

#[derive(Debug, Clone)]
pub struct NodeBox {
    pub id: NodeId,
    pub rect: Rectangle,
    /// The visible parent (the node's parent, or its nearest visible ancestor under a filter).
    pub parent: Option<NodeId>,
    /// Folded with children hidden by the fold.
    pub folded: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MapLayout {
    pub boxes: Vec<NodeBox>,
    index: HashMap<NodeId, usize>,
    /// The visible children of every laid out node, in display order.
    pub children: HashMap<NodeId, Vec<NodeId>>,
    pub size: Size,
}

impl MapLayout {
    pub fn get(&self, id: NodeId) -> Option<&NodeBox> {
        self.index.get(&id).map(|&i| &self.boxes[i])
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.index.contains_key(&id)
    }

    pub fn visible_children(&self, id: NodeId) -> &[NodeId] {
        self.children.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    fn depth(&self, id: NodeId) -> usize {
        std::iter::successors(self.get(id).and_then(|b| b.parent), |&p| self.get(p).and_then(|b| b.parent)).count()
    }

    /// The shown node above (`up`) or below `id` on its level: the previous or
    /// next sibling, and past the first or last sibling the nearest node of the
    /// same depth under a neighboring parent (an aunt's child, a cousin).
    pub fn neighbor_on_level(&self, id: NodeId, up: bool) -> Option<NodeId> {
        let depth = self.depth(id);
        // The boxes are in pre-order, so the nodes of one depth are in display order.
        let level: Vec<NodeId> = self.boxes.iter().map(|b| b.id).filter(|&n| self.depth(n) == depth).collect();
        let pos = level.iter().position(|&n| n == id)?;
        if up { pos.checked_sub(1) } else { Some(pos + 1) }.and_then(|p| level.get(p).copied())
    }
}

struct Ctx<'a> {
    map: &'a MindMap,
    visible: &'a HashSet<NodeId>,
    sizes: HashMap<NodeId, Size>,
    children: HashMap<NodeId, Vec<NodeId>>,
    heights: HashMap<NodeId, f32>,
    /// The distance from the top of a subtree's block to the node's center.
    centers: HashMap<NodeId, f32>,
    /// The distance from a node's center up to the top of its children's block.
    kids_above: HashMap<NodeId, f32>,
    h_gap: f32,
    v_gap: f32,
    rise: f32,
}

/// The children shown under `id`: visible children, and in place of a hidden
/// child its own shown children (a filter hides a node, not its visible
/// descendants). Nothing under a folded node.
fn shown_children(map: &MindMap, visible: &HashSet<NodeId>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    if map.is_folded(id) && id != map.root() {
        return out;
    }
    for c in map.children(id) {
        if visible.contains(&c) {
            out.push(c);
        } else {
            out.extend(shown_children(map, visible, c));
        }
    }
    out
}

impl Ctx<'_> {
    fn prepare(&mut self, id: NodeId, measure: &mut dyn FnMut(NodeId) -> Size) -> f32 {
        let size = measure(id);
        self.sizes.insert(id, size);
        let kids = shown_children(self.map, self.visible, id);
        let mut total = 0.0;
        for (i, &c) in kids.iter().enumerate() {
            if i > 0 {
                total += self.v_gap;
            }
            total += self.prepare(c, measure);
        }
        // The node is centered on its children's block; a single child is
        // placed a little higher than the node itself.
        let kids_above = match kids[..] {
            [] => 0.0,
            [c] => self.centers[&c] + self.rise,
            _ => total / 2.0,
        };
        self.children.insert(id, kids);
        let half = size.height / 2.0;
        let (above, below) = (half.max(kids_above), half.max(total - kids_above));
        self.centers.insert(id, above);
        self.kids_above.insert(id, kids_above);
        self.heights.insert(id, above + below);
        above + below
    }

    /// Places the subtree of `id` whose block starts at `top`, the node's left edge at `x`.
    fn place(&self, id: NodeId, x: f32, top: f32, parent: Option<NodeId>, out: &mut Vec<NodeBox>) {
        let size = self.sizes[&id];
        let center = top + self.centers[&id];
        let y = center - size.height / 2.0;
        out.push(NodeBox {
            id,
            rect: Rectangle::new(Point::new(x, y), size),
            parent,
            folded: self.map.is_folded(id) && self.map.has_children(id),
        });
        let kids = &self.children[&id];
        let mut cy = center - self.kids_above[&id];
        let cx = x + size.width + self.h_gap;
        for &c in kids {
            self.place(c, cx, cy, Some(id), out);
            cy += self.heights[&c] + self.v_gap;
        }
    }
}

/// Lays out the visible part of the map. `measure` gives a node's box size
/// (at the zoom), the gaps are scaled by `zoom`, and the margins are added
/// around the map: `before` left and above it, `after` right and below it.
pub fn layout(
    map: &MindMap,
    visible: &HashSet<NodeId>,
    (before, after): (Size, Size),
    zoom: f32,
    measure: &mut dyn FnMut(NodeId) -> Size,
) -> MapLayout {
    let mut ctx = Ctx {
        map,
        visible,
        sizes: HashMap::new(),
        children: HashMap::new(),
        heights: HashMap::new(),
        centers: HashMap::new(),
        kids_above: HashMap::new(),
        h_gap: H_GAP * zoom,
        v_gap: V_GAP * zoom,
        rise: SINGLE_CHILD_RISE * zoom,
    };
    // The root is laid out like any other node: its children to its right.
    ctx.prepare(map.root(), measure);
    let mut boxes = Vec::new();
    ctx.place(map.root(), 0.0, 0.0, None, &mut boxes);

    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for b in &boxes {
        min_x = min_x.min(b.rect.x);
        min_y = min_y.min(b.rect.y);
        max_x = max_x.max(b.rect.x + b.rect.width);
        max_y = max_y.max(b.rect.y + b.rect.height);
    }
    for b in &mut boxes {
        b.rect.x += before.width - min_x;
        b.rect.y += before.height - min_y;
    }
    let index = boxes.iter().enumerate().map(|(i, b)| (b.id, i)).collect();
    MapLayout {
        boxes,
        index,
        children: ctx.children,
        size: Size::new(max_x - min_x + before.width + after.width, max_y - min_y + before.height + after.height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_children_to_the_right_without_overlap() {
        let mut map = MindMap::new("root");
        let r = map.root();
        let a = map.create_child(r, None);
        let b = map.create_child(r, None);
        let a1 = map.create_child(a, None);
        let a2 = map.create_child(a, None);
        let visible: HashSet<NodeId> = map.preorder(r).into_iter().collect();
        let l =
            layout(&map, &visible, (Size::new(10.0, 10.0), Size::new(10.0, 10.0)), 1.0, &mut |_| Size::new(50.0, 20.0));
        assert_eq!(l.boxes.len(), 5);
        let root = l.get(r).unwrap().rect;
        let (ra, rb) = (l.get(a).unwrap().rect, l.get(b).unwrap().rect);
        assert!(ra.x > root.x + root.width && rb.x == ra.x);
        assert!(ra.y + ra.height <= rb.y);
        let (r1, r2) = (l.get(a1).unwrap().rect, l.get(a2).unwrap().rect);
        assert!(r1.x > ra.x + ra.width && r1.y + r1.height <= r2.y);
        assert!(l.boxes.iter().all(|b| b.rect.x >= 10.0 && b.rect.y >= 10.0));
        // The gaps grow with the zoom.
        let l2 =
            layout(&map, &visible, (Size::new(10.0, 10.0), Size::new(10.0, 10.0)), 2.0, &mut |_| Size::new(50.0, 20.0));
        let (root2, ra2) = (l2.get(r).unwrap().rect, l2.get(a).unwrap().rect);
        let (r12, r22) = (l2.get(a1).unwrap().rect, l2.get(a2).unwrap().rect);
        assert_eq!(ra2.x - (root2.x + root2.width), 2.0 * H_GAP);
        assert_eq!(r22.y - (r12.y + r12.height), 2.0 * V_GAP);
        // Folding hides the children.
        map.set_folded(a, true);
        let l = layout(&map, &visible, (Size::ZERO, Size::ZERO), 1.0, &mut |_| Size::new(50.0, 20.0));
        assert!(!l.contains(a1) && l.get(a).unwrap().folded);
    }

    #[test]
    fn a_single_child_is_raised() {
        let mut map = MindMap::new("root");
        let r = map.root();
        let b = map.create_child(r, None);
        let a = map.create_child(r, None);
        let b1 = map.create_child(b, None);
        let b2 = map.create_child(b, None);
        let a1 = map.create_child(a, None);
        let a11 = map.create_child(a1, None);
        let visible: HashSet<NodeId> = map.preorder(r).into_iter().collect();
        for zoom in [1.0, 2.0] {
            let l = layout(&map, &visible, (Size::ZERO, Size::ZERO), zoom, &mut |_| Size::new(50.0, 20.0) * zoom);
            let rect = |id| l.get(id).unwrap().rect;
            // The child node itself, not its subtree's block, is raised.
            assert_eq!(rect(a).y - rect(a1).y, SINGLE_CHILD_RISE * zoom);
            assert_eq!(rect(a1).y - rect(a11).y, SINGLE_CHILD_RISE * zoom);
            // Two children stay centered on their parent.
            assert_eq!(rect(b1).y + rect(b2).y + rect(b2).height, 2.0 * rect(b).y + rect(b).height);
            // The raised subtree keeps the gap to the one above.
            assert_eq!(rect(a11).y, rect(b2).y + rect(b2).height + V_GAP * zoom);
        }
        // A folded node shows no child to raise.
        map.set_folded(a, true);
        let l = layout(&map, &visible, (Size::ZERO, Size::ZERO), 1.0, &mut |_| Size::new(50.0, 20.0));
        let (ra, rb2) = (l.get(a).unwrap().rect, l.get(b2).unwrap().rect);
        assert_eq!(ra.y, rb2.y + rb2.height + V_GAP);
    }

    #[test]
    fn nodes_marked_left_are_laid_out_on_the_right() {
        let map = MindMap::parse(
            r#"<map version="freeplane 1.11.5">
<node TEXT="root" ID="ID_1">
<node TEXT="l" POSITION="left" ID="ID_2"/>
<node TEXT="r" POSITION="right" ID="ID_3"/>
</node>
</map>
"#,
        )
        .unwrap();
        let r = map.root();
        let visible: HashSet<NodeId> = map.preorder(r).into_iter().collect();
        let l = layout(&map, &visible, (Size::ZERO, Size::ZERO), 1.0, &mut |_| Size::new(50.0, 20.0));
        let root = l.get(r).unwrap().rect;
        assert!(map.children(r).iter().all(|&c| l.get(c).unwrap().rect.x > root.x + root.width));
        assert_eq!(l.visible_children(r), map.children(r).as_slice());
    }

    #[test]
    fn hidden_nodes_lift_their_visible_children() {
        let mut map = MindMap::new("root");
        let r = map.root();
        let a = map.create_child(r, None);
        let a1 = map.create_child(a, None);
        let visible: HashSet<NodeId> = [r, a1].into_iter().collect();
        let l = layout(&map, &visible, (Size::ZERO, Size::ZERO), 1.0, &mut |_| Size::new(50.0, 20.0));
        assert!(!l.contains(a));
        assert_eq!(l.get(a1).unwrap().parent, Some(r));
    }
}
