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

fn press(app: &mut App, id: window::Id, key: &str, modifiers: Modifiers) {
    send(app, Message::KeyIn(id, key_event(Key::Character(key.into()), modifiers)));
}

/// Selects `what` in the page (activating its block).
fn select(app: &mut App, id: window::Id, what: &str) {
    let text = page_text(app, id);
    let start = text.find(what).unwrap();
    editor(app, id, EditorMsg::PressedAt(start));
    let page = &mut app.editors.get_mut(&id).unwrap().page;
    page.place(crate::markdown::edit::Sel { anchor: start, head: start + what.len() });
}

#[test]
fn paragraph_shortcuts() {
    let (mut app, _f, _, _) = app_with_page("Plan\n\none\n\ntwo\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "Pl");
    press(&mut app, id, "2", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "## Plan\n\none\n\ntwo\n");
    press(&mut app, id, "=", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "# Plan\n\none\n\ntwo\n");
    press(&mut app, id, "0", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "Plan\n\none\n\ntwo\n");
    // Two blocks selected become a bullet list.
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    assert_eq!(app.editors[&id].page.blocks, Some((1, 2)));
    press(&mut app, id, "]", Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(page_text(&app, id), "Plan\n\n- one\n- two\n");
    click_after(&mut app, id, "two");
    press(&mut app, id, "q", Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(page_text(&app, id), "Plan\n\n- one\n> - two\n");
}

#[test]
fn format_shortcuts_and_clearing() {
    let (mut app, _f, _, _) = app_with_page("a word here\n");
    let id = open_with_f1(&mut app);
    select(&mut app, id, "word");
    press(&mut app, id, "b", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a **word** here\n");
    press(&mut app, id, "b", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a word here\n");
    // The word at the cursor.
    click_after(&mut app, id, "he");
    press(&mut app, id, "i", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a word *here*\n");
    select(&mut app, id, "word");
    press(&mut app, id, "k", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a [word]() *here*\n");
    // Ctrl+\ clears the formatting of the block.
    click_after(&mut app, id, "a ");
    press(&mut app, id, "\\", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a [word]() here\n");
}

#[test]
fn auto_pairing_can_be_turned_off() {
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    editor(&mut app, id, EditorMsg::Bind(Bind::Type(' ')));
    editor(&mut app, id, EditorMsg::Bind(Bind::Type('(')));
    assert_eq!(page_text(&app, id), "x ()\n");
    editor(&mut app, id, EditorMsg::Bind(Bind::Type(')')));
    assert_eq!(page_text(&app, id), "x ()\n");
    editor(&mut app, id, EditorMsg::Command(super::EditorCommand::ToggleAutoPair));
    assert!(!app.editor_prefs.auto_pair);
    editor(&mut app, id, EditorMsg::Bind(Bind::Type('[')));
    assert_eq!(page_text(&app, id), "x ()[\n");
}

#[test]
fn selection_helpers() {
    let (mut app, _f, _, _) = app_with_page("some **bold text** and more\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bo");
    press(&mut app, id, "e", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "bold text");
    click_after(&mut app, id, "mo");
    press(&mut app, id, "d", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "more");
    press(&mut app, id, "l", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "some **bold text** and more");
}

#[test]
fn ctrl_click_on_an_anchor_goes_to_the_heading() {
    let (mut app, _f, _, _) = app_with_page("[to b](#part-b)\n\n# Part A\n\n# Part B\n");
    let id = open_with_f1(&mut app);
    send(&mut app, Message::KeyIn(id, keyboard::Event::ModifiersChanged(Modifiers::CTRL)));
    editor(&mut app, id, EditorMsg::Link("#part-b".into(), 1));
    let page = &app.editors[&id].page;
    assert!(page.active.is_none());
    assert_eq!(page.sel.head, page.text.find("# Part B").unwrap());
}

fn clipboard(app: &App) -> super::clipboard::ClipboardContent {
    app.system_clipboard.lock().unwrap().read()
}

fn pasted(app: &mut App, id: window::Id, text: &str, html: Option<&str>, plain: bool) {
    let content = super::clipboard::ClipboardContent {
        text: Some(text.into()),
        html: html.map(str::to_string),
        ..Default::default()
    };
    editor(app, id, EditorMsg::Pasted(content, plain));
}

#[test]
fn copy_puts_markdown_and_html_on_the_clipboard() {
    let (mut app, _f, _, _) = app_with_page("a **bold** word\n");
    let id = open_with_f1(&mut app);
    select(&mut app, id, "**bold** word");
    press(&mut app, id, "c", Modifiers::CTRL);
    let c = clipboard(&app);
    assert_eq!(c.text.as_deref(), Some("**bold** word"));
    assert!(c.html.as_deref().is_some_and(|h| h.contains("<strong>bold</strong>")), "{c:?}");
    // Copy as Markdown: no HTML.
    press(&mut app, id, "c", Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(clipboard(&app).html, None);
    // Pasted back here: exactly the Markdown.
    click_after(&mut app, id, "word");
    pasted(&mut app, id, "**bold** word", Some("<strong>bold</strong> word"), false);
    assert_eq!(page_text(&app, id), "a **bold** word**bold** word\n");
}

#[test]
fn html_from_other_applications_is_pasted_as_markdown() {
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    pasted(&mut app, id, "bold and link", Some("<b>bold</b> and <a href=\"https://x.org\">link</a>"), false);
    assert_eq!(page_text(&app, id), "x**bold** and [link](https://x.org)\n");
    // Paste as plain text keeps the text.
    pasted(&mut app, id, " plain", Some("<b>plain</b>"), true);
    assert_eq!(page_text(&app, id), "x**bold** and [link](https://x.org) plain\n");
}

#[test]
fn a_spreadsheet_range_pasted_outside_a_table_becomes_a_table() {
    let (mut app, _f, _, _) = app_with_page("before\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "before");
    pasted(&mut app, id, "a\tb\n1\t2\n", None, false);
    assert_eq!(page_text(&app, id), "before\n\n| a   | b   |\n| --- | --- |\n| 1   | 2   |\n");
}

#[test]
fn cutting_whole_blocks() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    assert_eq!(app.editors[&id].page.blocks, Some((0, 1)));
    press(&mut app, id, "x", Modifiers::CTRL);
    assert_eq!(clipboard(&app).text.as_deref(), Some("one\n\ntwo"));
    assert_eq!(page_text(&app, id), "three\n");
}

#[test]
fn whole_blocks_are_selected_and_replaced() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "two");
    // Ctrl+A selects the block's text, a second time all blocks.
    press(&mut app, id, "a", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "two");
    press(&mut app, id, "a", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.blocks, Some((0, 2)));
    // Shift+click from the active block.
    click_after(&mut app, id, "one");
    send(&mut app, Message::KeyIn(id, keyboard::Event::ModifiersChanged(Modifiers::SHIFT)));
    editor(&mut app, id, EditorMsg::BlockPressed(1));
    assert_eq!(app.editors[&id].page.blocks, Some((0, 1)));
    send(&mut app, Message::KeyIn(id, keyboard::Event::ModifiersChanged(Modifiers::empty())));
    // Typing replaces them.
    send(
        &mut app,
        Message::KeyIn(
            id,
            keyboard::Event::KeyPressed {
                key: Key::Character("z".into()),
                modified_key: Key::Character("z".into()),
                physical_key: Physical::Unidentified(NativeCode::Unidentified),
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                text: Some("z".into()),
                repeat: false,
            },
        ),
    );
    assert_eq!(page_text(&app, id), "z\n\nthree\n");
}

#[test]
fn inserted_images_go_one_per_line() {
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    editor(
        &mut app,
        id,
        EditorMsg::Inserted(Ok(vec!["![](Plan.assets/a.png)".into(), "![](Plan.assets/b.jpg)".into()])),
    );
    assert_eq!(page_text(&app, id), "x\n\n![](Plan.assets/a.png)\n![](Plan.assets/b.jpg)\n");
    // Errors are shown.
    editor(&mut app, id, EditorMsg::Inserted(Err("failed to copy x".into())));
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Message("failed to copy x".into())));
}

#[test]
fn dropped_files_are_copied_into_the_assets_folder() {
    let (_app, f, _, page) = app_with_page("x\n");
    let picture = f.dir.path().join("photo.png");
    std::fs::write(&picture, b"png").unwrap();
    let pdf = f.dir.path().join("doc.pdf");
    std::fs::write(&pdf, b"pdf").unwrap();
    let opts = crate::markdown::images::ImageOptions::default();
    let md = super::files_markdown(&page, "x\n", &[picture, pdf], &opts).unwrap();
    assert_eq!(md, vec!["![](Plan.assets/photo.png)".to_string(), "[doc.pdf](../doc.pdf)".to_string()]);
    assert!(f.doc_dir.join("Plan.assets").join("photo.png").is_file());
}

fn find(app: &mut App, id: window::Id, msg: super::FindMsg) {
    editor(app, id, EditorMsg::Find(msg));
}

fn selected(app: &App, id: window::Id) -> std::ops::Range<usize> {
    app.editors[&id].page.sel.range()
}

#[test]
fn finding_the_next_and_previous_match() {
    let (mut app, _f, _, _) = app_with_page("Foo one\n\n- foo two\n- FOO three\n");
    let id = open_with_f1(&mut app);
    // Ctrl+F takes the selected text.
    select(&mut app, id, "Foo");
    press(&mut app, id, "f", Modifiers::CTRL);
    let bar = app.editors[&id].find.as_ref().expect("the find bar");
    assert_eq!(bar.query, "Foo");
    assert!(!bar.replacing);
    assert_eq!(bar.matches, vec![0..3, 11..14, 21..24]);
    assert_eq!(app.editors[&id].find_status(), "1 of 3");
    // F3 / Enter: the next match, selected in its block.
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::F3), Modifiers::empty())));
    assert_eq!(selected(&app, id), 11..14);
    assert_eq!(app.editors[&id].page.selected_source(), "foo");
    find(&mut app, id, super::FindMsg::Next);
    assert_eq!(selected(&app, id), 21..24);
    find(&mut app, id, super::FindMsg::Next);
    assert_eq!(selected(&app, id), 0..3, "wraps around");
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::F3), Modifiers::SHIFT)));
    assert_eq!(selected(&app, id), 21..24);
    // Match case.
    find(&mut app, id, super::FindMsg::MatchCase(true));
    assert_eq!(app.editors[&id].find.as_ref().unwrap().matches, vec![0..3]);
    // Esc closes the bar.
    editor(&mut app, id, EditorMsg::Escape);
    assert!(app.editors[&id].find.is_none());
}

#[test]
fn replace_and_replace_all() {
    let (mut app, _f, _, _) = app_with_page("foo a\n\nfoo b\n\nfoo c\n");
    let id = open_with_f1(&mut app);
    press(&mut app, id, "h", Modifiers::CTRL);
    assert!(app.editors[&id].find.as_ref().unwrap().replacing);
    find(&mut app, id, super::FindMsg::Query("foo".into()));
    find(&mut app, id, super::FindMsg::Replacement("bar".into()));
    // Replace without a selected match goes to the first; then replaces it.
    find(&mut app, id, super::FindMsg::Replace);
    assert_eq!(page_text(&app, id), "foo a\n\nfoo b\n\nfoo c\n");
    find(&mut app, id, super::FindMsg::Replace);
    assert_eq!(page_text(&app, id), "bar a\n\nfoo b\n\nfoo c\n");
    assert_eq!(app.editors[&id].page.selected_source(), "foo", "the next match");
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "foo a\n\nfoo b\n\nfoo c\n");
    // Replace All: one change.
    find(&mut app, id, super::FindMsg::ReplaceAll);
    assert_eq!(page_text(&app, id), "bar a\n\nbar b\n\nbar c\n");
    assert_eq!(app.editors[&id].find_status(), "No results");
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "foo a\n\nfoo b\n\nfoo c\n");
}

#[test]
fn finding_in_source_mode() {
    let (mut app, _f, _, _) = app_with_page("# foo\n\nfoo\n");
    let id = open_with_f1(&mut app);
    press(&mut app, id, "/", Modifiers::CTRL);
    press(&mut app, id, "f", Modifiers::CTRL);
    find(&mut app, id, super::FindMsg::Query("foo".into()));
    find(&mut app, id, super::FindMsg::Next);
    find(&mut app, id, super::FindMsg::Next);
    let source = app.editors[&id].source.as_ref().unwrap();
    assert_eq!(source.selection().as_deref(), Some("foo"));
    assert_eq!(super::source_sel(source, &source.text()).range(), 7..10);
    find(&mut app, id, super::FindMsg::Replacement("x".into()));
    find(&mut app, id, super::FindMsg::ReplaceAll);
    assert_eq!(page_text(&app, id), "# x\n\nx\n");
    assert_eq!(app.editors[&id].source.as_ref().unwrap().text(), "# x\n\nx\n");
}

const TABLE: &str = "| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n";

fn table_of(app: &App, id: window::Id) -> crate::markdown::table::Table {
    crate::markdown::table::Table::parse(&page_text(app, id)).expect("a table")
}

fn cell(app: &App, id: window::Id) -> (usize, usize) {
    app.editors[&id].page.active.as_ref().unwrap().table.as_ref().unwrap().cell
}

fn cell_range(app: &App, id: window::Id) -> Option<crate::markdown::table::CellRange> {
    app.editors[&id].page.active.as_ref().unwrap().table.as_ref().unwrap().range
}

fn rows(app: &App, id: window::Id) -> Vec<Vec<String>> {
    table_of(app, id).rows
}

fn named(app: &mut App, id: window::Id, key: Named, modifiers: Modifiers) {
    send(app, Message::KeyIn(id, key_event(Key::Named(key), modifiers)));
}

fn strings(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect()
}

#[test]
fn rows_are_inserted_deleted_and_moved_with_keys() {
    let (mut app, _f, _, _) = app_with_page(TABLE);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    assert_eq!(cell(&app, id), (1, 0));
    // Ctrl+Enter: an empty row below.
    named(&mut app, id, Named::Enter, Modifiers::CTRL);
    assert_eq!(rows(&app, id), strings(&[&["a", "b"], &["1", "2"], &["", ""], &["3", "4"]]));
    assert_eq!(cell(&app, id), (2, 0));
    // Ctrl+Shift+Backspace: deleted; the cursor in the row that followed.
    named(&mut app, id, Named::Backspace, Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(rows(&app, id), strings(&[&["a", "b"], &["1", "2"], &["3", "4"]]));
    assert_eq!(cell(&app, id), (2, 0));
    // Alt+Up moves the row.
    named(&mut app, id, Named::ArrowUp, Modifiers::ALT);
    assert_eq!(rows(&app, id), strings(&[&["a", "b"], &["3", "4"], &["1", "2"]]));
    assert_eq!(cell(&app, id), (1, 0));
    // The header stays where it is.
    named(&mut app, id, Named::ArrowUp, Modifiers::ALT);
    assert_eq!(rows(&app, id)[0], vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn moving_a_column_takes_its_alignment() {
    let (mut app, _f, _, _) = app_with_page("| a | b |\n| :-- | --: |\n| 1 | 2 |\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    named(&mut app, id, Named::ArrowRight, Modifiers::ALT);
    let t = table_of(&app, id);
    assert_eq!(t.rows, strings(&[&["b", "a"], &["2", "1"]]));
    use crate::markdown::table::Align;
    assert_eq!(t.align, vec![Align::Right, Align::Left]);
    assert_eq!(cell(&app, id), (1, 1));
}

#[test]
fn table_menu_commands() {
    let (mut app, _f, _, _) = app_with_page(TABLE);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "2");
    let cmd = |app: &mut App, c: super::EditorCommand| editor(app, id, EditorMsg::Command(c));
    cmd(&mut app, super::EditorCommand::ColumnLeft);
    assert_eq!(rows(&app, id), strings(&[&["a", "", "b"], &["1", "", "2"], &["3", "", "4"]]));
    cmd(&mut app, super::EditorCommand::DeleteColumn);
    cmd(&mut app, super::EditorCommand::ColumnRight);
    assert_eq!(rows(&app, id), strings(&[&["a", "b", ""], &["1", "2", ""], &["3", "4", ""]]));
    assert_eq!(cell(&app, id), (1, 2));
    cmd(&mut app, super::EditorCommand::RowAbove);
    assert_eq!(rows(&app, id)[1], vec![String::new(); 3]);
    cmd(&mut app, super::EditorCommand::DeleteRow);
    cmd(&mut app, super::EditorCommand::DeleteColumn);
    assert_eq!(rows(&app, id), strings(&[&["a", "b"], &["1", "2"], &["3", "4"]]));
    // Delete Table.
    cmd(&mut app, super::EditorCommand::DeleteTable);
    assert_eq!(page_text(&app, id), "");
}

#[test]
fn aligning_the_columns_of_a_range() {
    let (mut app, _f, _, _) = app_with_page(TABLE);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftRight));
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftRight));
    assert_eq!(cell_range(&app, id), Some(crate::markdown::table::CellRange::new((1, 0), (1, 1))));
    editor(&mut app, id, EditorMsg::Command(super::EditorCommand::AlignCenter));
    assert!(page_text(&app, id).contains("| :---: | :---: |"), "{}", page_text(&app, id));
    // The range stays for the next command.
    assert!(cell_range(&app, id).is_some());
}

#[test]
fn ranges_are_cleared_copied_and_pasted() {
    let (mut app, _f, _, _) = app_with_page(TABLE);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    // Dragging from a cell to another selects the cells between.
    editor(&mut app, id, EditorMsg::CellPressed(1, 0));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Entered(2, 1)));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Released));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Entered(1, 1)));
    let range = crate::markdown::table::CellRange::new((1, 0), (2, 1));
    assert_eq!(cell_range(&app, id), Some(range), "moving after the release changes nothing");
    press(&mut app, id, "c", Modifiers::CTRL);
    assert_eq!(clipboard(&app).text.as_deref(), Some("1\t2\n3\t4"));
    assert!(clipboard(&app).html.as_deref().is_some_and(|h| h.contains("<td>4</td>")));
    // Delete empties them.
    editor(&mut app, id, EditorMsg::Bind(Bind::Delete));
    assert_eq!(rows(&app, id), strings(&[&["a", "b"], &["", ""], &["", ""]]));
    assert_eq!(cell_range(&app, id), None);
    // A spreadsheet range pasted into the last row grows the table.
    editor(&mut app, id, EditorMsg::CellPressed(2, 0));
    pasted(&mut app, id, "x\ty\nz\tw\nu\tv\n", None, false);
    assert_eq!(rows(&app, id), strings(&[&["a", "b"], &["", ""], &["x", "y"], &["z", "w"], &["u", "v"]]));
}

#[test]
fn rows_and_columns_from_the_cell_menu() {
    let (mut app, _f, _, _) = app_with_page(TABLE);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::PointerAt(iced::Point::new(100.0, 200.0))));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Menu(2, 1)));
    assert_eq!(app.editors[&id].cell_menu, Some(iced::Point::new(100.0, 200.0)));
    assert_eq!(cell(&app, id), (2, 1), "the right-clicked cell");
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::MenuItem(super::keys::TableMenuItem::CopyColumn)));
    assert_eq!(clipboard(&app).text.as_deref(), Some("b\n2\n4"));
    assert_eq!(app.editors[&id].cell_menu, None);
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Menu(1, 0)));
    let item = super::keys::TableMenuItem::Command(super::EditorCommand::RowBelow);
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::MenuItem(item)));
    assert_eq!(rows(&app, id).len(), 4);
    // Ctrl+L selects the row.
    press(&mut app, id, "l", Modifiers::CTRL);
    assert_eq!(cell_range(&app, id), Some(crate::markdown::table::CellRange::new((2, 0), (2, 1))));
}

#[test]
fn resizing_from_the_toolbar() {
    let (mut app, _f, _, _) = app_with_page("| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    assert_eq!(app.editors[&id].resize_values(), ("2".to_string(), "2".to_string()));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Rows("3".into())));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Columns("3".into())));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Resize));
    assert_eq!(rows(&app, id), strings(&[&["a", "b", ""], &["1", "2", ""], &["3", "4", ""], &["", "", ""]]));
    // Not a number: nothing changes.
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Rows("x".into())));
    editor(&mut app, id, EditorMsg::Table(super::TableMsg::Resize));
    assert_eq!(rows(&app, id).len(), 4);
}

#[test]
fn typing_in_cells() {
    let (mut app, _f, _, _) = app_with_page("|a|b|\n|-|-|\n|1|2|\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "a");
    typing(&mut app, id, "x|y");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftEnter));
    typing(&mut app, id, "z");
    assert_eq!(rows(&app, id)[0][0], "ax\\|y<br>z");
    // Tab to the next cell selects its text; at the end a row is added.
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(app.editors[&id].page.selected_source(), "b");
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(rows(&app, id).len(), 3);
    assert_eq!(cell(&app, id), (2, 0));
    // Right at the end of a cell goes to the next one.
    typing(&mut app, id, "q");
    editor(&mut app, id, EditorMsg::Bind(Bind::Right));
    assert_eq!(cell(&app, id), (2, 1));
    editor(&mut app, id, EditorMsg::Bind(Bind::Left));
    assert_eq!(cell(&app, id), (2, 0));
    let text = page_text(&app, id);
    assert_eq!(text.lines().next(), Some("| ax\\|y<br>z | b   |"));
}
