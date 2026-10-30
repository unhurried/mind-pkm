//! What the tests driving the application by its messages share.

use super::{App, Message};
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};

/// Handles `message` as the running application would (the task is dropped).
pub fn send(app: &mut App, message: Message) {
    let _ = app.update(message);
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
