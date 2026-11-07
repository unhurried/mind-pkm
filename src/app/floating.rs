//! An element floating over the content it belongs to: the content is laid
//! out and takes its room as usual, the bar is shown just above it as an
//! overlay, taking no room - the page editor's table toolbar, so that going
//! into a table moves neither the table nor the blocks around it.
//!
//! The bar stays within the visible part it is given (the page's, as its
//! scrollable hands it): pushed down to its top while the content's top is
//! scrolled out of it, never below the content's bottom, and not shown when
//! it would not lie wholly in it (an overlay takes every press inside its
//! bounds, which must never be the menu bar's or the find bar's). Its widget
//! state stays with the widget while it is hidden.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, mouse};

pub struct Floating<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    bar: Element<'a, Message, Theme, Renderer>,
    gap: f32,
    shown: bool,
}

/// `content` with `bar` floating `gap` above it while `shown`.
pub fn floating<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    bar: impl Into<Element<'a, Message, Theme, Renderer>>,
    gap: f32,
    shown: bool,
) -> Floating<'a, Message, Theme, Renderer> {
    Floating { content: content.into(), bar: bar.into(), gap, shown }
}

/// Where a bar of `bar`'s size goes for content at `content` (both in the
/// coordinates of `visible`, the part shown): its bottom `gap` above the
/// content's top and its left at the content's left, pushed down to the top
/// of `visible` but no lower than the content's bottom, and moved left to
/// stay in it; `None` when it would not lie wholly within `visible`.
pub fn place(content: Rectangle, bar: Size, visible: Rectangle, gap: f32) -> Option<Point> {
    let x = content.x.min(visible.x + visible.width - bar.width).max(visible.x);
    let y = (content.y - gap - bar.height).max(visible.y).min(content.y + content.height - bar.height);
    let fits = y >= visible.y && y + bar.height <= visible.y + visible.height && bar.width <= visible.width;
    fits.then_some(Point::new(x, y))
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Floating<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.bar)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content, &self.bar]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
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
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        // The bar is operated on as an overlay.
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let [content_tree, bar_tree] = &mut tree.children[..] else {
            return None;
        };
        let content = self.content.as_widget_mut().overlay(content_tree, layout, renderer, viewport, translation);
        let bar = self.shown.then(|| {
            overlay::Element::new(Box::new(Bar {
                bar: &mut self.bar,
                tree: bar_tree,
                content: layout.bounds() + translation,
                visible: *viewport,
                gap: self.gap,
            }))
        });
        match (content, bar) {
            (Some(content), Some(bar)) => Some(overlay::Group::with_children(vec![content, bar]).overlay()),
            (content, bar) => content.or(bar),
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Floating<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(floating: Floating<'a, Message, Theme, Renderer>) -> Self {
        Element::new(floating)
    }
}

/// The bar as an overlay. Its layout holds the bar's as its one child, or
/// nothing when the bar does not fit in the visible part.
struct Bar<'a, 'b, Message, Theme, Renderer> {
    bar: &'a mut Element<'b, Message, Theme, Renderer>,
    tree: &'a mut Tree,
    /// The content's bounds and the visible part, on the window.
    content: Rectangle,
    visible: Rectangle,
    gap: f32,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer> for Bar<'_, '_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, _bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, Size::new(self.visible.width, f32::INFINITY));
        let node = self.bar.as_widget_mut().layout(self.tree, renderer, &limits);
        match place(self.content, node.size(), self.visible, self.gap) {
            Some(at) => layout::Node::with_children(node.size(), vec![node]).move_to(at),
            None => layout::Node::new(Size::ZERO),
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let Some(bar) = layout.children().next() {
            self.bar.as_widget().draw(self.tree, renderer, theme, style, bar, cursor, &bar.bounds());
        }
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        if let Some(bar) = layout.children().next() {
            self.bar.as_widget_mut().operate(self.tree, bar, renderer, operation);
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Some(bar) = layout.children().next() {
            let visible = self.visible;
            self.bar.as_widget_mut().update(self.tree, event, bar, cursor, renderer, clipboard, shell, &visible);
        }
    }

    /// Over the bar, never `None`: the widgets under it then get no pointer.
    fn mouse_interaction(&self, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer) -> mouse::Interaction {
        match layout.children().next() {
            Some(bar) if cursor.is_over(bar.bounds()) => {
                match self.bar.as_widget().mouse_interaction(self.tree, bar, cursor, &self.visible, renderer) {
                    mouse::Interaction::None => mouse::Interaction::Idle,
                    interaction => interaction,
                }
            }
            _ => mouse::Interaction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VISIBLE: Rectangle = Rectangle { x: 0.0, y: 100.0, width: 800.0, height: 500.0 };
    const BAR: Size = Size { width: 300.0, height: 20.0 };

    fn table(y: f32, height: f32) -> Rectangle {
        Rectangle { x: 40.0, y, width: 400.0, height }
    }

    #[test]
    fn the_bar_goes_just_above_the_content() {
        assert_eq!(place(table(300.0, 100.0), BAR, VISIBLE, 4.0), Some(Point::new(40.0, 276.0)));
    }

    #[test]
    fn the_bar_stays_at_the_top_of_the_visible_part_while_the_content_is_in_it() {
        // The table's top is out of view: the bar is at the top of what is shown.
        assert_eq!(place(table(50.0, 300.0), BAR, VISIBLE, 4.0), Some(Point::new(40.0, 100.0)));
        // Just room enough above the table.
        assert_eq!(place(table(124.0, 100.0), BAR, VISIBLE, 4.0), Some(Point::new(40.0, 100.0)));
        // As long as the bar does not go below the table's bottom.
        assert_eq!(place(table(0.0, 120.0), BAR, VISIBLE, 4.0), Some(Point::new(40.0, 100.0)));
    }

    #[test]
    fn no_bar_when_it_would_leave_the_visible_part() {
        // The table is scrolled out of view above, or almost.
        assert_eq!(place(table(0.0, 115.0), BAR, VISIBLE, 4.0), None);
        assert_eq!(place(table(-400.0, 100.0), BAR, VISIBLE, 4.0), None);
        // Below the visible part.
        assert_eq!(place(table(700.0, 100.0), BAR, VISIBLE, 4.0), None);
        // Narrower than the bar.
        assert_eq!(place(table(300.0, 100.0), BAR, Rectangle { width: 200.0, ..VISIBLE }, 4.0), None);
    }

    #[test]
    fn the_bar_moves_left_to_stay_in_view() {
        let wide = Rectangle { x: 600.0, ..table(300.0, 100.0) };
        assert_eq!(place(wide, BAR, VISIBLE, 4.0), Some(Point::new(500.0, 276.0)));
    }
}
