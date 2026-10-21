//! A wrapper that keeps the mouse pointer the normal arrow over everything it
//! contains - buttons, text boxes, the map and pop-up lists included - while
//! passing every event and drawing through unchanged. `hand_cursor` marks the
//! few widgets inside it that show a hand instead.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector, mouse};

/// What a hand-cursor widget reports to the arrow wrapper around it. No widget
/// of the app uses it otherwise, unlike `Pointer`, which every button reports.
const HAND_MARKER: mouse::Interaction = mouse::Interaction::Grab;

pub struct ArrowCursor<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    hand: bool,
}

pub fn arrow_cursor<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> ArrowCursor<'a, Message, Theme, Renderer> {
    ArrowCursor { content: content.into(), hand: false }
}

/// Shows the hand over `content`, where the surrounding `arrow_cursor` would
/// show the arrow.
pub fn hand_cursor<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> ArrowCursor<'a, Message, Theme, Renderer> {
    ArrowCursor { content: content.into(), hand: true }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ArrowCursor<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
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
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport)
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.hand {
            return if cursor.is_over(layout.bounds()) { HAND_MARKER } else { mouse::Interaction::None };
        }
        if self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer) == HAND_MARKER {
            return mouse::Interaction::Pointer;
        }
        // Both mean the arrow, but the shell only sets the pointer when the
        // value changes - and its initial value is None. Switching to Idle
        // when the pointer enters makes sure the arrow is actually set.
        if cursor.is_over(layout.bounds()) { mouse::Interaction::Idle } else { mouse::Interaction::None }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let inner = self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)?;
        Some(overlay::Element::new(Box::new(ArrowOverlay { content: inner })))
    }
}

/// The same for pop-ups (e.g. the open list of a drop-down).
struct ArrowOverlay<'a, Message, Theme, Renderer> {
    content: overlay::Element<'a, Message, Theme, Renderer>,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer> for ArrowOverlay<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        self.content.as_overlay_mut().layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_overlay().draw(renderer, theme, style, layout, cursor)
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_overlay_mut().operate(layout, renderer, operation)
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
        self.content.as_overlay_mut().update(event, layout, cursor, renderer, clipboard, shell)
    }

    fn mouse_interaction(&self, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer) -> mouse::Interaction {
        // Only where the pop-up itself takes the pointer: anything but None
        // makes the shell take the pointer away from the window underneath,
        // which would close a tooltip as soon as it opens.
        match self.content.as_overlay().mouse_interaction(layout, cursor, renderer) {
            mouse::Interaction::None => mouse::Interaction::None,
            _ => mouse::Interaction::Idle,
        }
    }

    fn overlay<'b>(
        &'b mut self,
        layout: Layout<'b>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let inner = self.content.as_overlay_mut().overlay(layout, renderer)?;
        Some(overlay::Element::new(Box::new(ArrowOverlay { content: inner })))
    }

    fn index(&self) -> f32 {
        self.content.as_overlay().index()
    }
}

impl<'a, Message, Theme, Renderer> From<ArrowCursor<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(widget: ArrowCursor<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
