//! A wrapper that sends a message once, when the window is drawn next - the
//! view built with it asks for another message (and so another view) right
//! after it is shown, while it passes everything else through unchanged.
//!
//! The page editor measures the blocks of a page a few at a time, a batch
//! with every view: while blocks are left, each view asks for the next. A
//! subscription could not do it, as iced asks for the subscriptions before
//! the views are built.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};

pub struct AfterDraw<'a, M> {
    content: Element<'a, M>,
    message: Option<M>,
}

/// `content`, sending `message` (if any) once, when the window is drawn next.
pub fn after_draw<'a, M>(content: impl Into<Element<'a, M>>, message: Option<M>) -> AfterDraw<'a, M> {
    AfterDraw { content: content.into(), message }
}

impl<M> Widget<M, Theme, Renderer> for AfterDraw<'_, M> {
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

    fn tag(&self) -> widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree)
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
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
        if let Event::Window(window::Event::RedrawRequested(_)) = event
            && let Some(message) = self.message.take()
        {
            shell.publish(message);
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

impl<'a, M: 'a> From<AfterDraw<'a, M>> for Element<'a, M> {
    fn from(widget: AfterDraw<'a, M>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::headless_renderer;
    use iced::advanced::clipboard;
    use iced::widget::space;
    use iced::{Point, Size};
    use iced_runtime::user_interface::{Cache, UserInterface};

    #[test]
    fn the_message_is_sent_once_when_the_window_is_drawn() {
        let mut renderer = headless_renderer();
        let size = Size::new(100.0, 100.0);
        let mut ui = UserInterface::build(after_draw(space(), Some(7)), size, Cache::default(), &mut renderer);
        let cursor = mouse::Cursor::Available(Point::ORIGIN);
        let redraw = Event::Window(window::Event::RedrawRequested(iced::time::Instant::now()));
        let moved = Event::Mouse(mouse::Event::CursorMoved { position: Point::ORIGIN });
        let mut messages: Vec<i32> = Vec::new();
        ui.update(&[moved], cursor, &mut renderer, &mut clipboard::Null, &mut messages);
        assert!(messages.is_empty());
        ui.update(std::slice::from_ref(&redraw), cursor, &mut renderer, &mut clipboard::Null, &mut messages);
        ui.update(&[redraw], cursor, &mut renderer, &mut clipboard::Null, &mut messages);
        assert_eq!(messages, [7]);
    }
}
