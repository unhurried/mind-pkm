//! A column whose children keep their widget state by key. iced's `Column`
//! hands the states to its children by their place: a child added or removed
//! in the middle hands every child after it the state of its neighbor, and
//! each of their texts is then shaped again. Here, before the column compares
//! itself with its new children, their states are put in the order of the
//! new keys: a child keeps the state its key had, wherever it moved, and a new
//! key gets a new state. Otherwise it is a `Column`: laid out, drawn and
//! handling events as one.
//!
//! It can also tell how tall each child was laid out (`on_layout`).
//!
//! The page editor shows the blocks of a page with it, so that a block added
//! or joined (Enter, Backspace) leaves the shaped text of the others as it is,
//! and keeps the heights of the blocks it built for those it does not build.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::widget::Column;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};
use std::collections::HashMap;
use std::hash::Hash;

/// Told the size of each child (by its key) as it was laid out.
type OnLayout<'a, K> = Box<dyn Fn(&K, Size) + 'a>;

pub struct KeyedColumn<'a, K, M> {
    keys: Vec<K>,
    column: Column<'a, M>,
    on_layout: Option<OnLayout<'a, K>>,
}

/// The keys of the children as last shown, in the order of the column's
/// children's states.
struct Keys<K>(Vec<K>);

/// The tag of no widget: the state of a key that was not there before, which
/// the column then makes anew for its child.
struct New;

/// An empty column.
pub fn keyed_column<'a, K, M: 'a>() -> KeyedColumn<'a, K, M> {
    KeyedColumn { keys: Vec::new(), column: Column::new(), on_layout: None }
}

impl<'a, K, M: 'a> KeyedColumn<'a, K, M> {
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.column = self.column.width(width);
        self
    }

    /// Tells `f` the size of each child (by its key) whenever the column is laid out.
    pub fn on_layout(mut self, f: impl Fn(&K, Size) + 'a) -> Self {
        self.on_layout = Some(Box::new(f));
        self
    }

    /// Adds `child` below the others; `key` names it among them.
    pub fn push(mut self, key: K, child: impl Into<Element<'a, M>>) -> Self {
        self.keys.push(key);
        self.column = self.column.push(child);
        self
    }
}

impl<K, M> KeyedColumn<'_, K, M> {
    fn column(&self) -> &dyn Widget<M, Theme, Renderer> {
        &self.column
    }
}

impl<K, M> Widget<M, Theme, Renderer> for KeyedColumn<'_, K, M>
where
    K: Copy + Eq + Hash + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Keys<K>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Keys(self.keys.clone()))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.column())]
    }

    /// Hands every child the state its key had, if it was there before: the
    /// column's own tree holds its children's states in their order, which
    /// is put in the order of the new keys before the column is compared
    /// (and, as always, a state whose widget is of another kind made anew).
    fn diff(&self, tree: &mut Tree) {
        let Tree { state, children, .. } = tree;
        let Keys(keys) = state.downcast_mut::<Keys<K>>();
        if let [column] = children.as_mut_slice()
            && *keys != self.keys
            && column.children.len() == keys.len()
        {
            let mut before: HashMap<K, Tree> = keys.drain(..).zip(column.children.drain(..)).collect();
            let new = || Tree { tag: tree::Tag::of::<New>(), state: tree::State::None, children: Vec::new() };
            column.children.extend(self.keys.iter().map(|key| before.remove(key).unwrap_or_else(new)));
        }
        keys.clone_from(&self.keys);
        tree.diff_children(&[self.column()]);
    }

    fn size(&self) -> Size<Length> {
        self.column.size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.column.size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let node = self.column.layout(&mut tree.children[0], renderer, limits);
        if let Some(f) = &self.on_layout {
            for (key, child) in self.keys.iter().zip(node.children()) {
                f(key, child.size());
            }
        }
        node
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.column.draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.column.operate(&mut tree.children[0], layout, renderer, operation)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        self.column.update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport)
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.column.mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.column.overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, K, M: 'a> From<KeyedColumn<'a, K, M>> for Element<'a, M>
where
    K: Copy + Eq + Hash + 'static,
{
    fn from(widget: KeyedColumn<'a, K, M>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::space;

    /// A widget whose state is the number it was made with.
    struct Numbered(u32);

    impl Widget<(), Theme, Renderer> for Numbered {
        fn tag(&self) -> tree::Tag {
            tree::Tag::of::<u32>()
        }
        fn state(&self) -> tree::State {
            tree::State::new(self.0)
        }
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fixed(10.0), Length::Fixed(self.0 as f32))
        }
        fn layout(&mut self, _: &mut Tree, _: &Renderer, _: &layout::Limits) -> layout::Node {
            layout::Node::new(Size::new(10.0, self.0 as f32))
        }
        fn draw(
            &self,
            _: &Tree,
            _: &mut Renderer,
            _: &Theme,
            _: &renderer::Style,
            _: Layout<'_>,
            _: mouse::Cursor,
            _: &Rectangle,
        ) {
        }
    }

    fn numbered<'a>(keys: &[u32], made_as: u32) -> KeyedColumn<'a, u32, ()> {
        keys.iter().fold(keyed_column(), |c, &k| c.push(k, Element::new(Numbered(k + made_as))))
    }

    /// The states of the column's children, in their order.
    fn states(tree: &Tree) -> Vec<u32> {
        tree.children[0].children.iter().map(|t| *t.state.downcast_ref::<u32>()).collect()
    }

    fn widget<'a>(c: &'a KeyedColumn<'static, u32, ()>) -> &'a dyn Widget<(), Theme, Renderer> {
        c
    }

    #[test]
    fn a_childs_state_stays_with_its_key() {
        let first = numbered(&[1, 2, 3, 4], 0);
        let mut tree = Tree::new(widget(&first));
        assert_eq!(states(&tree), [1, 2, 3, 4]);
        // One added in the middle: the children after it keep their states.
        let next = numbered(&[1, 5, 2, 3, 4], 100);
        tree.diff(widget(&next));
        assert_eq!(states(&tree), [1, 105, 2, 3, 4]);
        // Two removed and the rest moved around.
        let next = numbered(&[4, 1, 3], 100);
        tree.diff(widget(&next));
        assert_eq!(states(&tree), [4, 1, 3]);
        // A key twice: the second one gets a new state.
        let next = numbered(&[4, 4], 100);
        tree.diff(widget(&next));
        assert_eq!(states(&tree), [4, 104]);
    }

    #[test]
    fn a_child_of_another_kind_gets_a_new_state() {
        let first = numbered(&[1, 2], 0);
        let mut tree = Tree::new(widget(&first));
        let other = keyed_column::<u32, ()>().push(2, space()).push(1, Element::new(Numbered(7)));
        tree.diff(widget(&other));
        let column = &tree.children[0];
        assert_eq!(column.children.len(), 2);
        assert_ne!(column.children[0].tag, tree::Tag::of::<u32>());
        assert_eq!(*column.children[1].state.downcast_ref::<u32>(), 1);
    }

    #[test]
    fn it_is_laid_out_as_a_column_and_tells_its_childrens_sizes() {
        let sizes = std::cell::RefCell::new(Vec::new());
        let mut c = numbered(&[20, 30], 0).width(50).on_layout(|k, size| sizes.borrow_mut().push((*k, size.height)));
        let mut tree = Tree::new(&c as &dyn Widget<(), Theme, Renderer>);
        let renderer = crate::app::test_support::headless_renderer();
        let node = c.layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, Size::INFINITE));
        assert_eq!(node.size(), Size::new(50.0, 50.0));
        let ys: Vec<f32> = Layout::new(&node).children().map(|l| l.bounds().y).collect();
        assert_eq!(ys, [0.0, 20.0]);
        assert_eq!(*sizes.borrow(), [(20, 20.0), (30, 30.0)]);
    }
}
