//! What the tests driving the application by its messages share.

use super::{App, Message};
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};
use std::path::{Path, PathBuf};

/// The remover of the application in tests (`App::new`): instead of moving
/// files to the Recycle Bin, it removes them - only inside the temporary
/// directory, where the tests work. A file anywhere else fails the test.
pub fn remove_in_temp(files: &[PathBuf]) -> Result<(), String> {
    for f in files {
        assert!(in_temp(f), "a test removes {}, which is not inside the temporary directory", f.display());
        let removed = if f.is_dir() { std::fs::remove_dir_all(f) } else { std::fs::remove_file(f) };
        removed.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Whether `path` is an existing file or directory inside the temporary
/// directory (and not that directory itself).
fn in_temp(path: &Path) -> bool {
    let (Ok(temp), Ok(path)) = (std::env::temp_dir().canonicalize(), path.canonicalize()) else { return false };
    path.starts_with(&temp) && path != temp
}

#[test]
fn the_test_remover_removes_only_inside_the_temporary_directory() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Plan.md");
    std::fs::write(&file, "x\n").unwrap();
    assert!(in_temp(&file) && in_temp(dir.path()));
    let temp = std::env::temp_dir();
    assert!(!in_temp(&temp) && !in_temp(temp.parent().unwrap()));
    assert!(!in_temp(&dir.path().join("missing.md")));
    remove_in_temp(std::slice::from_ref(&file)).unwrap();
    assert!(!file.exists());
}

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
