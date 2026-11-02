//! Widgets at given places in an area of a given size, layered like a stack -
//! each above the ones before it: it takes events first, and while it reacts
//! to the pointer over it, those below do not see the pointer - and each with
//! a key. When the widgets change from one view to the next, a widget's state
//! (its shaped text, a button's hover, a click counted towards a double click)
//! stays with its key, wherever the widget moved in the list.
//!
//! The map shows its nodes with it, only those near the visible part of the
//! map: a set that changes as the map scrolls, while the nodes that stay keep
//! their state.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use std::collections::HashMap;
use std::hash::Hash;

pub struct Placed<'a, K, M> {
    size: Size,
    keys: Vec<K>,
    points: Vec<Point>,
    children: Vec<Element<'a, M>>,
}

/// The keys of the widgets as last shown, in the order of the tree's children.
struct Keys<K>(Vec<K>);

/// An area of `size` with no widgets yet.
pub fn placed<'a, K, M>(size: Size) -> Placed<'a, K, M> {
    Placed { size, keys: Vec::new(), points: Vec::new(), children: Vec::new() }
}

impl<'a, K, M> Placed<'a, K, M> {
    /// Adds `child` with its top left corner at `at`, above the widgets added
    /// before it. `key` names it among them.
    pub fn push(&mut self, key: K, at: Point, child: impl Into<Element<'a, M>>) {
        self.keys.push(key);
        self.points.push(at);
        self.children.push(child.into());
    }

    #[cfg(test)]
    pub fn keys(&self) -> &[K] {
        &self.keys
    }
}

impl<K, M> Widget<M, Theme, Renderer> for Placed<'_, K, M>
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
        self.children.iter().map(|child| Tree::new(child.as_widget())).collect()
    }

    /// Hands every widget the state its key had, if it was there before.
    fn diff(&self, tree: &mut Tree) {
        let Tree { state, children, .. } = tree;
        let Keys(keys) = state.downcast_mut::<Keys<K>>();
        let mut before: HashMap<K, Tree> = keys.drain(..).zip(children.drain(..)).collect();
        children.extend(self.keys.iter().zip(&self.children).map(|(key, child)| match before.remove(key) {
            Some(mut state) => {
                state.diff(child.as_widget());
                state
            }
            None => Tree::new(child.as_widget()),
        }));
        keys.clone_from(&self.keys);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size.width), Length::Fixed(self.size.height))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let size = limits.resolve(self.size.width, self.size.height, Size::ZERO);
        let nodes = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(&self.points)
            .map(|((child, tree), &at)| {
                let room = Size::new((size.width - at.x).max(0.0), (size.height - at.y).max(0.0));
                child.as_widget_mut().layout(tree, renderer, &layout::Limits::new(Size::ZERO, room)).move_to(at)
            })
            .collect();
        layout::Node::with_children(size, nodes)
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
        if !layout.bounds().intersects(viewport) {
            return;
        }
        // The widgets below the topmost one reacting to the pointer don't see it.
        let under = if cursor.is_over(layout.bounds()) {
            self.children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .rposition(|((child, tree), layout)| {
                    child.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
                        != mouse::Interaction::None
                })
                .unwrap_or(0)
        } else {
            0
        };
        for (i, ((child, tree), layout)) in self.children.iter().zip(&tree.children).zip(layout.children()).enumerate()
        {
            if layout.bounds().intersects(viewport) {
                let cursor = if i < under { mouse::Cursor::Unavailable } else { cursor };
                child.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
            }
        }
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, tree), layout) in self.children.iter_mut().zip(&mut tree.children).zip(layout.children()) {
                child.as_widget_mut().operate(tree, layout, renderer, operation);
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        mut cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let is_over = cursor.is_over(layout.bounds());
        let last = self.children.len().saturating_sub(1);
        let children = self.children.iter_mut().rev().zip(tree.children.iter_mut().rev()).zip(layout.children().rev());
        for (i, ((child, tree), layout)) in children.enumerate() {
            child.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
            if shell.is_event_captured() {
                return;
            }
            if i < last
                && is_over
                && !cursor.is_levitating()
                && child.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
                    != mouse::Interaction::None
            {
                cursor = cursor.levitate();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .rev()
            .zip(tree.children.iter().rev())
            .zip(layout.children().rev())
            .map(|((child, tree), layout)| {
                child.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .find(|&interaction| interaction != mouse::Interaction::None)
            .unwrap_or_default()
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        overlay::from_children(&mut self.children, tree, layout, renderer, viewport, translation)
    }
}

impl<'a, K, M: 'a> From<Placed<'a, K, M>> for Element<'a, M>
where
    K: Copy + Eq + Hash + 'static,
{
    fn from(widget: Placed<'a, K, M>) -> Self {
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
            Size::new(Length::Shrink, Length::Shrink)
        }
        fn layout(&mut self, _: &mut Tree, _: &Renderer, _: &layout::Limits) -> layout::Node {
            layout::Node::new(Size::ZERO)
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

    fn numbered(keys: &[u32], made_as: u32) -> Placed<'static, u32, ()> {
        let mut p = placed(Size::new(100.0, 100.0));
        for &k in keys {
            p.push(k, Point::ORIGIN, Element::new(Numbered(k + made_as)));
        }
        p
    }

    fn states(tree: &Tree) -> Vec<u32> {
        tree.children.iter().map(|t| *t.state.downcast_ref::<u32>()).collect()
    }

    #[test]
    fn a_widgets_state_stays_with_its_key() {
        let first = numbered(&[1, 2, 3], 0);
        let mut tree = Tree::new(&first as &dyn Widget<(), Theme, Renderer>);
        assert_eq!(states(&tree), vec![1, 2, 3]);
        // 1 goes, 4 comes, 3 moves before 2: the states made for 2 and 3 stay
        // theirs, 4 gets a new one.
        let next = numbered(&[3, 4, 2], 100);
        tree.diff(&next as &dyn Widget<(), Theme, Renderer>);
        assert_eq!(states(&tree), vec![3, 104, 2]);
        assert_eq!(next.keys(), &[3, 4, 2]);
        // A widget of another kind under a key gets a new state.
        let mut other = placed::<u32, ()>(Size::new(100.0, 100.0));
        other.push(3, Point::ORIGIN, space());
        tree.diff(&other as &dyn Widget<(), Theme, Renderer>);
        assert_eq!(tree.children.len(), 1);
        assert_ne!(tree.children[0].tag, tree::Tag::of::<u32>());
    }

    #[test]
    fn widgets_are_laid_out_at_their_places() {
        let mut p = placed::<u32, ()>(Size::new(300.0, 200.0));
        p.push(1, Point::new(10.0, 20.0), space().width(50).height(30));
        p.push(2, Point::new(250.0, 150.0), space().width(100).height(100));
        let mut tree = Tree::new(&p as &dyn Widget<(), Theme, Renderer>);
        let renderer = crate::app::test_support::headless_renderer();
        let node = p.layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, Size::INFINITE));
        assert_eq!(node.size(), Size::new(300.0, 200.0));
        let bounds: Vec<Rectangle> = Layout::new(&node).children().map(|l| l.bounds()).collect();
        assert_eq!(bounds[0], Rectangle::new(Point::new(10.0, 20.0), Size::new(50.0, 30.0)));
        // Kept inside the area.
        assert_eq!(bounds[1], Rectangle::new(Point::new(250.0, 150.0), Size::new(50.0, 50.0)));
    }
}
