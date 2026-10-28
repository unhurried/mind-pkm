//! The topmost layer of a window while a dialog is open: it takes the
//! keyboard (and the text an input method commits) away from the widgets
//! below it. Those would otherwise keep their focus and edit the page behind
//! the dialog. The keys are handed on as messages, so that the dialog can
//! still be answered with them.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector, keyboard, mouse};

pub struct Modal<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    on_key: Box<dyn Fn(keyboard::Event) -> Message + 'a>,
}

/// Wraps `content` so that no keyboard event goes past it: its key events are
/// published as `on_key(event)`, and its own widgets (buttons) see them first.
pub fn modal<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_key: impl Fn(keyboard::Event) -> Message + 'a,
) -> Modal<'a, Message, Theme, Renderer> {
    Modal { content: content.into(), on_key: Box::new(on_key) }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Modal<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
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
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
        match event {
            Event::Keyboard(key) => {
                if !shell.is_event_captured() {
                    shell.publish((self.on_key)(key.clone()));
                }
                shell.capture_event();
            }
            Event::InputMethod(_) => shell.capture_event(),
            _ => {}
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
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Modal<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(modal: Modal<'a, Message, Theme, Renderer>) -> Self {
        Element::new(modal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::clipboard;
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Key, Location, Modifiers, key::Named};
    use iced::widget::{button, space, stack};

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Key(Key),
        Pressed,
        Below,
    }

    fn press(key: Key) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }

    /// A stack of a layer that reports the keys it gets (`Below`) and a modal
    /// one above it: the messages published for `event`, and whether it was captured.
    fn send(event: Event) -> (Vec<Msg>, bool) {
        struct Below;
        impl Widget<Msg, iced::Theme, ()> for Below {
            fn size(&self) -> Size<Length> {
                Size::new(Length::Fill, Length::Fill)
            }
            fn layout(&mut self, _: &mut Tree, _: &(), limits: &layout::Limits) -> layout::Node {
                layout::atomic(limits, Length::Fill, Length::Fill)
            }
            fn draw(
                &self,
                _: &Tree,
                _: &mut (),
                _: &iced::Theme,
                _: &renderer::Style,
                _: Layout<'_>,
                _: mouse::Cursor,
                _: &Rectangle,
            ) {
            }
            fn update(
                &mut self,
                _: &mut Tree,
                event: &Event,
                _: Layout<'_>,
                _: mouse::Cursor,
                _: &(),
                _: &mut dyn Clipboard,
                shell: &mut Shell<'_, Msg>,
                _: &Rectangle,
            ) {
                if matches!(event, Event::Keyboard(_) | Event::InputMethod(_)) {
                    shell.publish(Msg::Below);
                }
            }
        }
        let dialog: Element<'_, Msg, iced::Theme, ()> =
            button(space()).on_press_with(|| Msg::Pressed).width(50).height(20).into();
        let mut ui: Element<'_, Msg, iced::Theme, ()> = stack![
            Element::new(Below),
            modal(dialog, |e| match e {
                keyboard::Event::KeyPressed { key, .. } => Msg::Key(key),
                _ => Msg::Pressed,
            })
        ]
        .into();
        let mut tree = Tree::new(&ui);
        let node = ui.as_widget_mut().layout(&mut tree, &(), &layout::Limits::new(Size::ZERO, Size::new(200.0, 100.0)));
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        ui.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &(),
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::new(iced::Point::ORIGIN, Size::new(200.0, 100.0)),
        );
        let captured = shell.is_event_captured();
        (messages, captured)
    }

    #[test]
    fn keys_do_not_reach_the_layers_below_but_are_published() {
        let (messages, captured) = send(press(Key::Named(Named::Enter)));
        assert_eq!(messages, vec![Msg::Key(Key::Named(Named::Enter))]);
        assert!(captured);
    }

    #[test]
    fn typed_characters_are_kept_from_the_layers_below() {
        let (messages, captured) = send(press(Key::Character("L".into())));
        assert_eq!(messages, vec![Msg::Key(Key::Character("L".into()))]);
        assert!(captured);
    }

    #[test]
    fn text_from_an_input_method_is_kept_from_the_layers_below() {
        let (messages, captured) = send(Event::InputMethod(iced::advanced::input_method::Event::Commit("あ".into())));
        assert_eq!(messages, vec![]);
        assert!(captured);
    }

    #[test]
    fn other_events_are_left_alone() {
        let (messages, captured) = send(Event::Mouse(mouse::Event::CursorLeft));
        assert_eq!(messages, vec![]);
        assert!(!captured);
    }
}
