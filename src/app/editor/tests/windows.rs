//! Editor windows: opening pages from the map, keys reaching their window, saving, closing and leaving the application, dialogs, scrolling.

use super::*;

#[test]
fn f1_opens_a_page_in_an_editor_window_once() {
    let (mut app, _f, _, page) = app_with_page("# Plan\n");
    let id = open_with_f1(&mut app);
    assert_eq!(app.editors.len(), 1);
    assert_eq!(app.editors[&id].page.path(), page);
    assert_eq!(app.title(id), "Plan.md - Mind PKM");
    // Again: the same window comes to the front.
    open_with_f1(&mut app);
    assert_eq!(app.editors.len(), 1);
}

#[test]
fn pages_open_in_the_editor_unless_it_is_turned_off() {
    let dir = tempfile::tempdir().unwrap();
    let md = dir.path().join("a.md");
    std::fs::write(&md, "x").unwrap();
    assert!(opens_in_editor(&md, true));
    assert!(!opens_in_editor(&md, false));
    assert!(!opens_in_editor(&dir.path().join("a.pdf"), true));
    assert!(!opens_in_editor(dir.path(), true));
    let mut app = App::new();
    assert!(app.editor_prefs.builtin);
    send(&mut app, MapMsg::Command(Command::ToggleBuiltinEditor));
    assert!(!app.editor_prefs.builtin);
}

#[test]
fn keys_go_to_the_window_they_were_typed_in() {
    let (mut app, _f, node, _) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    let main = app.main_window;
    // F7 in the map window toggles the checkmark; in the editor it does nothing to the map.
    send(&mut app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(Named::F7), Modifiers::empty()))));
    let map = &app.doc.as_ref().unwrap().map();
    assert!(!map.has_icon(node, crate::mindmap::ICON_CHECKMARK));
    send(&mut app, Message::Window(main, WindowEvent::Key(key_event(Key::Named(Named::F7), Modifiers::empty()))));
    assert!(app.doc.as_ref().unwrap().map().has_icon(node, crate::mindmap::ICON_CHECKMARK));
    // Ctrl+S in the editor saves the page, not the map.
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    send(&mut app, Message::Window(id, WindowEvent::Key(key_event(Key::Character("s".into()), Modifiers::CTRL))));
    assert!(!app.editors[&id].page.is_dirty());
    assert!(app.doc.as_ref().unwrap().dirty());
}

#[test]
fn saving_keeps_the_bom_and_crlf() {
    let (mut app, _f, _, page) = app_with_page("# Title\r\n\r\nsome words\r\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "some");
    typing(&mut app, id, "X");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Save));
    assert_eq!(std::fs::read(&page).unwrap(), "\u{feff}# Title\r\n\r\nsomeX words\r\n".as_bytes());
}

#[test]
fn the_saved_message_names_the_file_and_goes_on_the_next_edit() {
    let (mut app, _f, _, page) = app_with_page("some words\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "some");
    typing(&mut app, id, "X");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Save));
    let name = page.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(app.editors[&id].status.text(), format!("saved {name}"));
    typing(&mut app, id, "Y");
    assert_eq!(app.editors[&id].status.text(), "");
}

#[test]
fn closing_an_edited_page_asks_first() {
    let (mut app, _f, _, page) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    send(&mut app, Message::Window(id, WindowEvent::CloseRequested));
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Unsaved));
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Cancel));
    assert!(app.editors.contains_key(&id));
    assert_eq!(page_text(&app, id), "one!\n");
    send(&mut app, Message::Window(id, WindowEvent::CloseRequested));
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Discard));
    assert!(!app.editors.contains_key(&id));
    assert_eq!(std::fs::read_to_string(page).unwrap(), "\u{feff}one\n");
}

#[test]
fn exiting_with_unsaved_pages_asks_and_cancel_keeps_them() {
    let (mut app, _f, _, page) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    send(&mut app, MapMsg::CloseRequested);
    assert!(matches!(app.dialogs.front(), Some(Dialog::UnsavedPages(p)) if p == &vec!["Plan.md".to_string()]));
    send(&mut app, MapMsg::DialogCancel);
    assert!(app.dialogs.is_empty());
    assert!(app.editors[&id].page.is_dirty());
    // Save writes the page (and ends the application).
    send(&mut app, MapMsg::CloseRequested);
    send(&mut app, MapMsg::DialogSave);
    assert_eq!(std::fs::read_to_string(page).unwrap(), "\u{feff}one!\n");
    assert!(app.dialogs.is_empty());
}

#[test]
fn cancelling_the_pages_question_after_dont_save_keeps_the_map_unsaved() {
    let (mut app, _f, _, _) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    app.doc.as_mut().unwrap().change(|_| ());
    send(&mut app, MapMsg::CloseRequested);
    assert!(matches!(app.dialogs.front(), Some(Dialog::Unsaved(AfterUnsaved::Exit))));
    send(&mut app, MapMsg::DialogDiscard);
    assert!(matches!(app.dialogs.front(), Some(Dialog::UnsavedPages(_))));
    send(&mut app, MapMsg::DialogCancel);
    assert!(app.dialogs.is_empty());
    // Cancel aborts the whole exit: the map is still unsaved and is asked about again.
    assert!(app.doc.as_ref().unwrap().dirty());
    send(&mut app, MapMsg::CloseRequested);
    assert!(matches!(app.dialogs.front(), Some(Dialog::Unsaved(AfterUnsaved::Exit))));
}

#[test]
fn ctrl_click_on_a_link_to_another_page_opens_it() {
    let (mut app, f, _, _) = app_with_page("see [other](Other.md)\n");
    let other = f.write_page("Other", "x");
    let id = open_with_f1(&mut app);
    // A plain click edits the link; Ctrl+click follows it.
    editor(&mut app, id, EditorMsg::Link("Other.md".into(), 5));
    assert_eq!(app.editors.len(), 1);
    send(&mut app, Message::Window(id, WindowEvent::Key(keyboard::Event::ModifiersChanged(Modifiers::CTRL))));
    editor(&mut app, id, EditorMsg::Link("Other.md".into(), 5));
    assert_eq!(app.editors.len(), 2);
    assert!(app.editors.values().any(|e| e.page.path() == other));
}

#[test]
fn a_page_that_is_not_utf8_is_reported() {
    let (mut app, f, _, _) = app_with_page("x");
    std::fs::write(f.doc_dir.join("Plan.md"), [0x93, 0xfa, 0x96, 0x7b]).unwrap();
    send(&mut app, MapMsg::Key(key_event(Key::Named(Named::F1), Modifiers::empty())));
    assert!(app.editors.is_empty());
    assert!(matches!(app.dialogs.front(), Some(Dialog::Message(m)) if m.starts_with("failed to open")));
}

#[test]
fn outline_zoom_theme_and_full_screen() {
    let (mut app, _f, _, _) = app_with_page("# A\n\ntext\n\n## B\n\nmore\n");
    let id = open_with_f1(&mut app);
    // The outline lists the headings; a click goes to one.
    press(&mut app, id, "1", Modifiers::CTRL | Modifiers::SHIFT);
    assert!(app.editors[&id].outline);
    assert_eq!(app.editors[&id].page.doc().headings.len(), 2);
    editor(&mut app, id, EditorMsg::HeadingClicked(1));
    assert_eq!(app.editors[&id].page.sel().head, page_text(&app, id).find("## B").unwrap());
    // Zoom steps and back, remembered with the theme in the preferences.
    press(&mut app, id, "=", Modifiers::CTRL | Modifiers::SHIFT);
    assert!(app.editor_prefs.zoom > 1.0);
    press(&mut app, id, "0", Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(app.editor_prefs.zoom, 1.0);
    press(&mut app, id, "-", Modifiers::CTRL | Modifiers::SHIFT);
    assert!(app.editor_prefs.zoom < 1.0);
    editor(&mut app, id, EditorMsg::Command(EditorCommand::DarkTheme));
    assert!(app.editor_prefs.dark);
    assert_eq!(app.theme(id), iced::Theme::Dark);
    // F11 full screen, Esc leaves it.
    send(&mut app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(Named::F11), Modifiers::empty()))));
    assert!(app.editors[&id].fullscreen);
    editor(&mut app, id, EditorMsg::Escape);
    assert!(!app.editors[&id].fullscreen);
}

#[test]
fn enter_chooses_the_default_button_of_a_dialog() {
    let (mut app, _f, _, page) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    send(&mut app, Message::Window(id, WindowEvent::CloseRequested));
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Unsaved));
    // Other keys do nothing while the dialog is open.
    editor(&mut app, id, EditorMsg::Key(key_event(Key::Character("x".into()), Modifiers::empty())));
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Unsaved));
    // Enter saves, and the window closes.
    editor(&mut app, id, EditorMsg::Key(key_event(Key::Named(Named::Enter), Modifiers::empty())));
    assert!(!app.editors.contains_key(&id));
    assert_eq!(std::fs::read_to_string(page).unwrap(), "\u{feff}one!\n");
}

#[test]
fn enter_reloads_a_page_changed_on_disk_and_closes_a_message() {
    let (mut app, _f, _, page) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    let enter = || EditorMsg::Key(key_event(Key::Named(Named::Enter), Modifiers::empty()));
    app.editors.get_mut(&id).unwrap().dialogs.push_back(EditorDialog::ChangedOnDisk);
    std::fs::write(&page, "\u{feff}two\n").unwrap();
    editor(&mut app, id, enter());
    assert!(app.editors[&id].dialogs.is_empty());
    assert_eq!(page_text(&app, id), "two\n");
    app.editors.get_mut(&id).unwrap().dialogs.push_back(EditorDialog::Message("oops".into()));
    editor(&mut app, id, enter());
    assert!(app.editors[&id].dialogs.is_empty());
}

#[test]
fn the_diagram_being_edited_is_keyed_like_its_render() {
    use crate::app::editor::render::{RenderKey, keys_of};
    let (mut app, _f, _, _) = app_with_page("x\n\n```mermaid\ngraph TD\nA-->B\n```\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "A-->B");
    let e = &app.editors[&id];
    let a = e.page.active().unwrap();
    let key = RenderKey::Mermaid(e.active_diagram(a));
    assert!(keys_of(e.page.doc(), false).contains(&key), "{key:?}");
}

#[test]
fn only_moving_the_cursor_or_editing_scrolls_to_it() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    let prefs = app.editor_prefs.clone();
    let update =
        |app: &mut App, msg: EditorMsg| app.editors.get_mut(&id).unwrap().update(msg, &prefs, &mut Vec::new()).units();
    // A key no binding moves the cursor with (a released key, a modifier):
    // the page stays scrolled where the user left it.
    let released = keyboard::Event::KeyReleased {
        key: Key::Named(Named::Shift),
        modified_key: Key::Named(Named::Shift),
        physical_key: Physical::Unidentified(NativeCode::Unidentified),
        location: Location::Left,
        modifiers: Modifiers::empty(),
    };
    assert_eq!(update(&mut app, EditorMsg::Key(released)), 0);
    assert_eq!(update(&mut app, EditorMsg::Key(keyboard::Event::ModifiersChanged(Modifiers::SHIFT))), 0);
    // Moving the cursor brings it into view.
    assert!(update(&mut app, EditorMsg::Bind(Bind::Down)) > 0);
}
