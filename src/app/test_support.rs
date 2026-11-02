//! What the tests driving the application by its messages share.

use super::{App, Message};
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};

/// A renderer without a window (tiny-skia), for laying out and drawing widgets.
pub fn headless_renderer() -> iced::Renderer {
    use iced::advanced::renderer::Headless;
    let renderer = <iced::Renderer as Headless>::new(iced::Font::DEFAULT, iced::Pixels(16.0), Some("tiny-skia"));
    tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(renderer).expect("a headless renderer")
}

/// Handles `message` as the running application would (the task is dropped).
pub fn send(app: &mut App, message: impl Into<Message>) {
    let _ = app.update(message.into());
}

/// Types `text` into the node being edited, replacing what is there.
pub fn type_node_text(app: &mut App, text: &str) {
    use iced::widget::text_editor::{Action, Edit};
    send(app, super::MapMsg::NodeEdit(Action::SelectAll));
    send(app, super::MapMsg::NodeEdit(Action::Edit(Edit::Paste(std::sync::Arc::new(text.to_string())))));
}

/// A key pressed with `modifiers`, as a window reports it (typing no text).
pub fn key_event(key: Key, modifiers: Modifiers) -> keyboard::Event {
    key_press(key, modifiers, None)
}

/// A key pressed with `modifiers` that types `text`.
pub fn typing_event(key: Key, modifiers: Modifiers, text: &str) -> keyboard::Event {
    key_press(key, modifiers, Some(text))
}

fn key_press(key: Key, modifiers: Modifiers, text: Option<&str>) -> keyboard::Event {
    keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: Physical::Unidentified(NativeCode::Unidentified),
        location: Location::Standard,
        modifiers,
        text: text.map(Into::into),
        repeat: false,
    }
}
