//! Where the right mouse button goes down. `right_press_at` wraps a window's
//! content and publishes the press's place in the window before the widgets
//! under the pointer handle it (they say which context menu opens, which then
//! opens there). `right_press_in` wraps one widget and publishes the press's
//! place in it, minus its padding, when the widget itself did not take it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

pub struct RightPress<'a, M> {
    content: Element<'a, M>,
    on_press: Box<dyn Fn(Point) -> M + 'a>,
    /// None: the place in the window, published first; Some: the place in
    /// the content inside this padding, published after it and taken.
    inner: Option<Padding>,
}

/// Publishes where the right button goes down over `content`, in the window,
/// before `content` handles the press.
pub fn right_press_at<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    on_press: impl Fn(Point) -> M + 'a,
) -> RightPress<'a, M> {
    RightPress { content: content.into(), on_press: Box::new(on_press), inner: None }
}

/// Publishes where the right button goes down over `content`, relative to
/// its area inside `padding`, unless `content` took the press.
pub fn right_press_in<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    padding: Padding,
    on_press: impl Fn(Point) -> M + 'a,
) -> RightPress<'a, M> {
    RightPress { content: content.into(), on_press: Box::new(on_press), inner: Some(padding) }
}

impl<M> Widget<M, Theme, Renderer> for RightPress<'_, M> {
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
        let pressed = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)));
        if pressed
            && self.inner.is_none()
            && let Some(p) = cursor.position_over(bounds)
        {
            shell.publish((self.on_press)(p));
        }
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
        if pressed
            && let Some(padding) = self.inner
            && !shell.is_event_captured()
            && let Some(p) = cursor.position_over(bounds)
        {
            shell.publish((self.on_press)(Point::new(p.x - bounds.x - padding.left, p.y - bounds.y - padding.top)));
            shell.capture_event();
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

impl<'a, M: 'a> From<RightPress<'a, M>> for Element<'a, M> {
    fn from(widget: RightPress<'a, M>) -> Self {
        Element::new(widget)
    }
}
