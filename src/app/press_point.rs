//! Where a mouse button goes down, told only when it does. A widget that
//! needs to know where a click lands in it (to map it back to the source, to
//! start a drag or open a menu there) need not follow every move of the
//! pointer: each move would be a message, after which every window is built
//! again. `press_at` tells the place in the window, `press_in` the place in
//! the content; both before the content handles the press, which they leave
//! to it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

pub struct PressPoint<'a, M> {
    content: Element<'a, M>,
    on_press: Box<dyn Fn(Point) -> M + 'a>,
    /// The place in the window, else in the content.
    in_window: bool,
}

/// Publishes where the left or right button goes down over `content`, in the
/// window, before `content` handles the press.
pub fn press_at<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    on_press: impl Fn(Point) -> M + 'a,
) -> PressPoint<'a, M> {
    PressPoint { content: content.into(), on_press: Box::new(on_press), in_window: true }
}

/// Publishes where the left or right button goes down over `content`,
/// relative to its top left corner, before `content` handles the press.
pub fn press_in<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    on_press: impl Fn(Point) -> M + 'a,
) -> PressPoint<'a, M> {
    PressPoint { content: content.into(), on_press: Box::new(on_press), in_window: false }
}

impl<M> Widget<M, Theme, Renderer> for PressPoint<'_, M> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree)
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
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
        self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation)
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
        let bounds = layout.bounds();
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left | mouse::Button::Right)) = event
            && let Some(p) = cursor.position_over(bounds)
        {
            let p = if self.in_window { p } else { Point::new(p.x - bounds.x, p.y - bounds.y) };
            shell.publish((self.on_press)(p));
        }
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, M: 'a> From<PressPoint<'a, M>> for Element<'a, M> {
    fn from(widget: PressPoint<'a, M>) -> Self {
        Element::new(widget)
    }
}
