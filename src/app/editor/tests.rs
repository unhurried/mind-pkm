//! Editor windows driven by their messages, without windows: opening pages
//! from the map, typing, saving, closing, and leaving the application.

use super::keys::Bind;
use super::{DialogAnswer, EditorDialog, EditorMsg, opens_in_editor};
use crate::app::command::Command;
use crate::app::dialog::Dialog;
use crate::app::{App, Message};
use crate::mindmap::NodeId;
use crate::pkm::testutil::Fixture;
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers, key::Named};
use iced::widget::text_editor::{Action, Edit};
use iced::window;
use std::path::PathBuf;

fn send(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn key_event(key: Key, modifiers: Modifiers) -> keyboard::Event {
    keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: Physical::Unidentified(NativeCode::Unidentified),
        location: Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

/// An application showing a map with a page node `Plan` whose page holds `text`.
fn app_with_page(text: &str) -> (App, Fixture, NodeId, PathBuf) {
    let mut f = Fixture::new();
    let root = f.map.root();
    let page = f.write_page("Plan", text);
    let node = f.add_page_node(root, "Plan");
    let mut app = App::new();
    let _ = app.set_doc(f.map.clone(), None);
    app.doc.as_mut().unwrap().select(vec![node]);
    (app, f, node, page)
}

/// Opens the selected page with F1; its window's id.
fn open_with_f1(app: &mut App) -> window::Id {
    send(app, Message::Key(key_event(Key::Named(Named::F1), Modifiers::empty())));
    *app.editors.keys().last().expect("an editor window")
}

fn editor(app: &mut App, id: window::Id, msg: EditorMsg) {
    send(app, Message::Editor(id, msg));
}

fn typing(app: &mut App, id: window::Id, s: &str) {
    for c in s.chars() {
        editor(app, id, EditorMsg::Action(Action::Edit(Edit::Insert(c))));
    }
}

fn page_text(app: &App, id: window::Id) -> String {
    app.editors[&id].page.text.clone()
}

/// Puts the cursor at the end of `after` in the page.
fn click_after(app: &mut App, id: window::Id, after: &str) {
    let at = page_text(app, id).find(after).unwrap() + after.len();
    editor(app, id, EditorMsg::PressedAt(at));
}

#[test]
fn f1_opens_a_page_in_an_editor_window_once() {
    let (mut app, _f, _, page) = app_with_page("# Plan\n");
    let id = open_with_f1(&mut app);
    assert_eq!(app.editors.len(), 1);
    assert_eq!(app.editors[&id].page.path, page);
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
    send(&mut app, Message::Command(Command::ToggleBuiltinEditor));
    assert!(!app.editor_prefs.builtin);
}

#[test]
fn keys_go_to_the_window_they_were_typed_in() {
    let (mut app, _f, node, _) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    let main = app.main_window;
    // F7 in the map window toggles the checkmark; in the editor it does nothing to the map.
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::F7), Modifiers::empty())));
    let map = &app.doc.as_ref().unwrap().map;
    assert!(!map.has_icon(node, crate::mindmap::ICON_CHECKMARK));
    send(&mut app, Message::KeyIn(main, key_event(Key::Named(Named::F7), Modifiers::empty())));
    assert!(app.doc.as_ref().unwrap().map.has_icon(node, crate::mindmap::ICON_CHECKMARK));
    // Ctrl+S in the editor saves the page, not the map.
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    send(&mut app, Message::KeyIn(id, key_event(Key::Character("s".into()), Modifiers::CTRL)));
    assert!(!app.editors[&id].page.is_dirty());
    assert!(app.doc.as_ref().unwrap().dirty);
}

#[test]
fn typing_enter_and_undo() {
    let (mut app, _f, _, _) = app_with_page("- one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "two");
    assert_eq!(page_text(&app, id), "- one\n- two\n");
    assert_eq!(app.title(id), "Plan.md * - Mind PKM");
    editor(&mut app, id, EditorMsg::Command(super::EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "- one\n- \n");
    editor(&mut app, id, EditorMsg::Command(super::EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "- one\n");
}

#[test]
fn saving_keeps_the_bom_and_crlf() {
    let (mut app, _f, _, page) = app_with_page("# Title\r\n\r\nsome words\r\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "some");
    typing(&mut app, id, "X");
    editor(&mut app, id, EditorMsg::Command(super::EditorCommand::Save));
    assert_eq!(std::fs::read(&page).unwrap(), "\u{feff}# Title\r\n\r\nsomeX words\r\n".as_bytes());
}

#[test]
fn closing_an_edited_page_asks_first() {
    let (mut app, _f, _, page) = app_with_page("one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    typing(&mut app, id, "!");
    send(&mut app, Message::CloseRequest(id));
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Unsaved));
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Cancel));
    assert!(app.editors.contains_key(&id));
    assert_eq!(page_text(&app, id), "one!\n");
    send(&mut app, Message::CloseRequest(id));
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
    send(&mut app, Message::CloseRequested);
    assert!(matches!(app.dialogs.front(), Some(Dialog::UnsavedPages(p)) if p == &vec!["Plan.md".to_string()]));
    send(&mut app, Message::DialogCancel);
    assert!(app.dialogs.is_empty());
    assert!(app.editors[&id].page.is_dirty());
    // Save writes the page (and ends the application).
    send(&mut app, Message::CloseRequested);
    send(&mut app, Message::DialogSave);
    assert_eq!(std::fs::read_to_string(page).unwrap(), "\u{feff}one!\n");
    assert!(app.dialogs.is_empty());
}

#[test]
fn ctrl_click_on_a_link_to_another_page_opens_it() {
    let (mut app, f, _, _) = app_with_page("see [other](Other.md)\n");
    let other = f.write_page("Other", "x");
    let id = open_with_f1(&mut app);
    // A plain click edits the link; Ctrl+click follows it.
    editor(&mut app, id, EditorMsg::Link("Other.md".into(), 5));
    assert_eq!(app.editors.len(), 1);
    send(&mut app, Message::KeyIn(id, keyboard::Event::ModifiersChanged(Modifiers::CTRL)));
    editor(&mut app, id, EditorMsg::Link("Other.md".into(), 5));
    assert_eq!(app.editors.len(), 2);
    assert!(app.editors.values().any(|e| e.page.path == other));
}

#[test]
fn a_page_that_is_not_utf8_is_reported() {
    let (mut app, f, _, _) = app_with_page("x");
    std::fs::write(f.doc_dir.join("Plan.md"), [0x93, 0xfa, 0x96, 0x7b]).unwrap();
    send(&mut app, Message::Key(key_event(Key::Named(Named::F1), Modifiers::empty())));
    assert!(app.editors.is_empty());
    assert!(matches!(app.dialogs.front(), Some(Dialog::Message(m)) if m.starts_with("failed to open")));
}

#[test]
fn an_empty_template_item_is_typed_into() {
    let (mut app, _f, _, _) = app_with_page("### Next Steps\r\n\r\n* \r\n");
    let id = open_with_f1(&mut app);
    // A click beside the item's (missing) text, on its bullet.
    let item = app.editors[&id].page.doc.blocks.len() - 1;
    editor(&mut app, id, EditorMsg::BlockPressed(item));
    typing(&mut app, id, "step");
    assert_eq!(page_text(&app, id), "### Next Steps\n\n* step\n");
}

#[test]
fn arrows_cross_into_the_next_and_previous_blocks() {
    let (mut app, _f, _, _) = app_with_page("first line\n\nsecond\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "first");
    editor(&mut app, id, EditorMsg::Bind(Bind::Down));
    let page = &app.editors[&id].page;
    assert_eq!(page.region_text(), Some("second"));
    editor(&mut app, id, EditorMsg::Bind(Bind::Up));
    assert_eq!(app.editors[&id].page.region_text(), Some("first line"));
    // Right at the end and Left at the start cross too.
    click_after(&mut app, id, "first line");
    editor(&mut app, id, EditorMsg::Bind(Bind::Right));
    assert_eq!(app.editors[&id].page.region_text(), Some("second"));
    editor(&mut app, id, EditorMsg::Bind(Bind::Left));
    assert_eq!(app.editors[&id].page.region_text(), Some("first line"));
}

#[test]
fn backspace_at_the_start_joins_paragraphs() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n");
    let id = open_with_f1(&mut app);
    editor(&mut app, id, EditorMsg::PressedAt(5));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "onetwo\n");
    assert_eq!(app.editors[&id].page.sel.head, 3);
}

#[test]
fn a_task_box_is_checked_by_a_click() {
    let (mut app, _f, _, _) = app_with_page("- [ ] call\n");
    let id = open_with_f1(&mut app);
    editor(&mut app, id, EditorMsg::TaskToggled(0));
    assert_eq!(page_text(&app, id), "- [x] call\n");
    editor(&mut app, id, EditorMsg::Command(super::EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "- [ ] call\n");
}
