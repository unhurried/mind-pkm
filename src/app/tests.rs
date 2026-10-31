//! The application driven by its messages, without a window: what the user
//! does (keys, commands, dialog answers) and what arrives from the background.

use super::command::Command;
use super::dialog::Dialog;
use super::file::{AfterUnsaved, FileAction};
use super::test_support::{key_event, send};
use super::{App, MapMsg, settings};
use crate::mindmap::MindMap;
use crate::pkm::filter::Filter;
use crate::pkm::next_steps;
use crate::pkm::testutil::Fixture;
use iced::keyboard::{Key, Modifiers, key::Named};

fn app_with(map: MindMap) -> App {
    let mut app = App::new();
    let _ = app.set_doc(map, None);
    app
}

fn press(app: &mut App, key: Named) {
    send(app, MapMsg::Key(key_event(Key::Named(key), Modifiers::empty())));
}

fn map(app: &App) -> &MindMap {
    app.doc.as_ref().unwrap().map()
}

fn root_texts(app: &App) -> Vec<String> {
    map(app).child_texts(map(app).root())
}

#[test]
fn a_new_node_asks_for_its_text_and_undo_removes_it() {
    let mut app = app_with(MindMap::new("root"));
    press(&mut app, Named::Tab);
    assert!(matches!(app.dialogs.front(), Some(Dialog::Input { .. })));
    send(&mut app, MapMsg::DialogInput("child".into()));
    press(&mut app, Named::Enter);
    assert!(app.dialogs.is_empty());
    assert_eq!(root_texts(&app), vec!["child"]);
    assert!(app.doc.as_ref().unwrap().dirty());
    send(&mut app, MapMsg::Command(Command::Undo));
    assert!(root_texts(&app).is_empty());
}

#[test]
fn cancelling_a_new_node_leaves_the_map_as_it_was() {
    let mut app = app_with(MindMap::new("root"));
    let before = map(&app).to_xml();
    press(&mut app, Named::Tab);
    send(&mut app, MapMsg::Escape);
    assert!(app.dialogs.is_empty());
    assert_eq!(map(&app).to_xml(), before);
    // Still saved, and no refresh is called for.
    let doc = app.doc.as_ref().unwrap();
    assert!(!doc.dirty() && !doc.edited_since_refresh());
    // Leaving the text empty removes it too.
    press(&mut app, Named::Enter);
    send(&mut app, MapMsg::DialogOk);
    assert_eq!(map(&app).to_xml(), before);
}

#[test]
fn a_refresh_landing_while_a_new_node_is_named_is_kept() {
    let mut f = Fixture::new();
    let root = f.map.root();
    f.write_page("Task", "### Next Steps\n* One\n");
    let page = f.add_page_node(root, "Task");
    let update = next_steps::compute(&next_steps::snapshot(&f.map, &f.doc_dir), chrono::Local::now().date_naive());
    let mut app = app_with(f.map.clone());
    press(&mut app, Named::Tab);
    let generation = app.doc.as_ref().unwrap().generation();
    send(&mut app, MapMsg::NextStepsComputed(generation, Box::new(update)));
    send(&mut app, MapMsg::Escape);
    assert_eq!(map(&app).child_texts(page), vec!["One"]);
    assert_eq!(root_texts(&app), vec!["config", "ToDo", "Task"]);
    // The refresh is no undo step, and no undo step brings the new node back.
    send(&mut app, MapMsg::Command(Command::Undo));
    assert_eq!(map(&app).child_texts(page), vec!["One"]);
    assert_eq!(root_texts(&app), vec!["config", "ToDo", "Task"]);
}

#[test]
fn a_new_page_that_fails_changes_nothing() {
    let mut f = Fixture::new();
    let root = f.map.root();
    f.add_child(root, "bad/name");
    let mut app = app_with(f.map.clone());
    let bad = map(&app).find_child_by_text(root, "bad/name").unwrap();
    app.doc.as_mut().unwrap().select(vec![bad]);
    for command in [Command::NewPage, Command::NewDirectory] {
        send(&mut app, MapMsg::Command(command));
    }
    let doc = app.doc.as_ref().unwrap();
    assert!(!doc.dirty() && !doc.edited_since_refresh());
    // No empty undo step either: Undo has nothing to take back.
    let before = map(&app).to_xml();
    send(&mut app, MapMsg::Command(Command::Undo));
    assert_eq!(map(&app).to_xml(), before);
    assert!(!app.doc.as_ref().unwrap().dirty());
}

#[test]
fn a_new_page_that_already_exists_links_the_node_as_a_change_to_undo() {
    let mut f = Fixture::new();
    let root = f.map.root();
    f.write_page("Existing", "# Existing\n");
    f.add_child(root, "Existing");
    let mut app = app_with(f.map.clone());
    let node = map(&app).find_child_by_text(root, "Existing").unwrap();
    let before = map(&app).to_xml();
    assert_eq!(map(&app).link_file(node), None);
    app.doc.as_mut().unwrap().select(vec![node]);
    send(&mut app, MapMsg::Command(Command::NewPage));
    // Linked, and the user is told so ...
    assert!(map(&app).link_file(node).is_some());
    assert!(matches!(app.dialogs.front(), Some(Dialog::Message(m)) if m == "page file already exists."));
    // ... as a change: the map is unsaved, and Undo takes the link back.
    assert!(app.doc.as_ref().unwrap().dirty());
    send(&mut app, MapMsg::Command(Command::Undo));
    assert_eq!(map(&app).to_xml(), before);
}

#[test]
fn undo_selects_what_was_selected_before_the_change() {
    let mut app = app_with(MindMap::new("root"));
    press(&mut app, Named::Tab);
    send(&mut app, MapMsg::DialogInput("a".into()));
    press(&mut app, Named::Enter);
    let a = app.doc.as_ref().unwrap().focus();
    press(&mut app, Named::Tab);
    send(&mut app, MapMsg::DialogInput("b".into()));
    press(&mut app, Named::Enter);
    send(&mut app, MapMsg::Command(Command::Undo));
    assert_eq!(app.doc.as_ref().unwrap().focus(), a);
}

#[test]
fn a_refresh_keeps_matching_children_and_the_selection() {
    let mut f = Fixture::new();
    let root = f.map.root();
    f.write_page("Task", "### Next Steps\n* step A\n* step B\n");
    let page = f.add_page_node(root, "Task");
    let a = f.add_child(page, "step A");
    f.add_child(page, "step B");
    f.map.add_icon(a, "button_ok");
    let mut app = app_with(f.map.clone());
    app.doc.as_mut().unwrap().select(vec![a]);
    f.write_page("Task", "### Next Steps\n* step A\n* step B\n* step C\n");
    let update = next_steps::compute(&next_steps::snapshot(map(&app), &f.doc_dir), chrono::Local::now().date_naive());
    let generation = app.doc.as_ref().unwrap().generation();
    send(&mut app, MapMsg::NextStepsComputed(generation, Box::new(update)));
    assert_eq!(map(&app).child_texts(page), vec!["step A", "step B", "step C"]);
    assert_eq!(map(&app).children(page)[0], a);
    assert!(map(&app).has_icon(a, "button_ok"));
    assert_eq!(app.doc.as_ref().unwrap().focus(), a);
}

#[test]
fn a_node_added_under_a_page_node_is_warned_about() {
    let mut f = Fixture::new();
    let root = f.map.root();
    f.write_page("Task", "### Next Steps\n* One\n");
    let page = f.add_page_node(root, "Task");
    let mut app = app_with(f.map.clone());
    app.doc.as_mut().unwrap().select(vec![page]);
    press(&mut app, Named::Tab);
    assert_eq!(app.status.text(), super::command::UNDER_PAGE_WARNING);
    send(&mut app, MapMsg::Escape);
    // Not under a page node: no warning.
    app.status.clear();
    app.doc.as_mut().unwrap().select(vec![root]);
    press(&mut app, Named::Tab);
    assert_eq!(app.status.text(), "");
}

fn edited_app() -> App {
    let mut app = app_with(MindMap::new("root"));
    press(&mut app, Named::Tab);
    send(&mut app, MapMsg::DialogInput("x".into()));
    send(&mut app, MapMsg::DialogOk);
    app
}

#[test]
fn closing_with_unsaved_changes_asks_and_cancel_keeps_them() {
    let mut app = edited_app();
    send(&mut app, MapMsg::CloseRequested);
    assert!(matches!(app.dialogs.front(), Some(Dialog::Unsaved(AfterUnsaved::Exit))));
    // Enter doesn't answer this question; Cancel and Esc keep the map.
    press(&mut app, Named::Enter);
    assert_eq!(app.dialogs.len(), 1);
    send(&mut app, MapMsg::DialogCancel);
    assert!(app.dialogs.is_empty());
    assert_eq!(root_texts(&app), vec!["x"]);
    assert!(app.doc.as_ref().unwrap().dirty());
    send(&mut app, MapMsg::CloseRequested);
    send(&mut app, MapMsg::Escape);
    assert!(app.dialogs.is_empty());
    assert_eq!(root_texts(&app), vec!["x"]);
}

#[test]
fn new_map_with_unsaved_changes_can_discard_them() {
    let mut app = edited_app();
    send(&mut app, MapMsg::Command(Command::New));
    assert!(matches!(app.dialogs.front(), Some(Dialog::Unsaved(AfterUnsaved::New))));
    send(&mut app, MapMsg::DialogDiscard);
    let doc = app.doc.as_ref().unwrap();
    assert!(!doc.dirty() && doc.map().path().is_none());
    assert_eq!(doc.map().text(doc.map().root()), "Map");
    assert_eq!(root_texts(&app), vec!["config", "ToDo"]);
}

#[test]
fn saving_a_new_map_from_the_unsaved_changes_question_goes_on_once_saved() {
    let mut app = edited_app();
    send(&mut app, MapMsg::Command(Command::New));
    // The map has no file yet: Save asks for one first.
    send(&mut app, MapMsg::DialogSave);
    assert!(app.dialogs.is_empty());
    assert_eq!(root_texts(&app), vec!["x"]);
    let dir = tempfile::tempdir().unwrap();
    send(&mut app, MapMsg::FileChosen(FileAction::SaveAs(Some(AfterUnsaved::New)), Some(dir.path().join("saved"))));
    let saved = MindMap::load(&dir.path().join("saved.mm")).unwrap();
    assert_eq!(saved.child_texts(saved.root()), vec!["x"]);
    // Then the new map.
    assert!(map(&app).path().is_none());
    assert_eq!(root_texts(&app), vec!["config", "ToDo"]);
}

#[test]
fn cancelling_save_as_from_the_unsaved_changes_question_keeps_the_changes() {
    let mut app = edited_app();
    send(&mut app, MapMsg::CloseRequested);
    send(&mut app, MapMsg::DialogSave);
    send(&mut app, MapMsg::FileChosen(FileAction::SaveAs(Some(AfterUnsaved::Exit)), None));
    assert!(app.doc.as_ref().unwrap().dirty());
    assert_eq!(root_texts(&app), vec!["x"]);
}

#[test]
fn the_filter_dialog_sets_the_filter_and_cancel_keeps_it() {
    let mut app = app_with(MindMap::new("root"));
    assert_eq!(*app.doc.as_ref().unwrap().filter(), Filter::HideArchive);
    press(&mut app, Named::F5);
    send(&mut app, MapMsg::DialogInput("t".into()));
    send(&mut app, MapMsg::DialogOk);
    assert_eq!(*app.doc.as_ref().unwrap().filter(), Filter::ToDo);
    press(&mut app, Named::F5);
    send(&mut app, MapMsg::Escape);
    assert_eq!(*app.doc.as_ref().unwrap().filter(), Filter::ToDo);
    assert!(app.dialogs.is_empty());
}

#[test]
fn typing_in_the_search_box_reaches_the_map_only_through_function_keys() {
    let mut app = app_with(MindMap::new("root"));
    app.search.open = true;
    app.search.focus = super::search_panel::PanelFocus::Input;
    let before = map(&app).to_xml();
    for key in [Named::Tab, Named::Enter, Named::Delete, Named::Space] {
        press(&mut app, key);
    }
    assert!(app.dialogs.is_empty());
    assert_eq!(map(&app).to_xml(), before);
    press(&mut app, Named::F7);
    let root = map(&app).root();
    assert!(map(&app).has_icon(root, crate::mindmap::ICON_CHECKMARK));
}

fn press_char(app: &mut App, c: &str, modifiers: Modifiers) {
    send(app, MapMsg::Key(key_event(Key::Character(c.into()), modifiers)));
}

#[test]
fn ctrl_shift_f_asks_for_a_filter_like_f5() {
    let mut app = app_with(MindMap::new("root"));
    press_char(&mut app, "f", Modifiers::CTRL | Modifiers::SHIFT);
    send(&mut app, MapMsg::DialogInput("t".into()));
    send(&mut app, MapMsg::DialogOk);
    assert_eq!(*app.doc.as_ref().unwrap().filter(), Filter::ToDo);
}

#[test]
fn in_the_search_box_ctrl_f_reaches_the_map_but_other_ctrl_keys_do_not() {
    let mut app = app_with(MindMap::new("root"));
    app.search.open = true;
    app.search.focus = super::search_panel::PanelFocus::Input;
    press_char(&mut app, "n", Modifiers::CTRL);
    press_char(&mut app, "o", Modifiers::CTRL);
    assert!(app.dialogs.is_empty());
    // The map has no document directory, so the search command reports it.
    press_char(&mut app, "f", Modifiers::CTRL);
    assert!(!app.dialogs.is_empty());
}

#[test]
fn folding_by_click_is_one_undo_step() {
    let mut m = MindMap::new("root");
    let root = m.root();
    let a = m.add_child(root, "a");
    m.add_child(a, "a1");
    let mut app = app_with(m);
    send(&mut app, MapMsg::FoldClicked(a));
    assert!(map(&app).is_folded(a));
    assert!(!app.doc.as_ref().unwrap().layout().contains(map(&app).children(a)[0]));
    send(&mut app, MapMsg::Command(Command::Undo));
    assert!(!map(&app).is_folded(a));
}

#[test]
fn up_and_down_cross_to_cousins() {
    let mut m = MindMap::new("root");
    let root = m.root();
    let a = m.add_child(root, "a");
    let a1 = m.add_child(a, "a1");
    let a2 = m.add_child(a, "a2");
    let b = m.add_child(root, "b");
    let b1 = m.add_child(b, "b1");
    let mut app = app_with(m);
    app.doc.as_mut().unwrap().select(vec![a2]);
    press(&mut app, Named::ArrowDown);
    assert_eq!(app.doc.as_ref().unwrap().focus(), b1);
    press(&mut app, Named::ArrowDown);
    assert_eq!(app.doc.as_ref().unwrap().focus(), b1);
    press(&mut app, Named::ArrowUp);
    assert_eq!(app.doc.as_ref().unwrap().focus(), a2);
    press(&mut app, Named::ArrowUp);
    assert_eq!(app.doc.as_ref().unwrap().focus(), a1);
    press(&mut app, Named::ArrowUp);
    assert_eq!(app.doc.as_ref().unwrap().focus(), a1);
}

#[test]
fn a_map_opens_as_it_was_last_shown() {
    let mut f = Fixture::new();
    let root = f.map.root();
    let a = f.add_child(root, "a");
    let b = f.add_child(a, "b");
    let c = f.add_child(root, "c");
    let mut app = app_with(f.map.clone());
    send(&mut app, MapMsg::Command(Command::ZoomIn));
    send(&mut app, MapMsg::Command(Command::ZoomIn));
    app.doc.as_mut().unwrap().select(vec![c, b]);
    app.viewport.scroll = iced::Vector::new(30.0, 40.0);
    let (path, view) = app.current_view().unwrap();
    assert_eq!(path, f.dir.path().join("map.mm"));
    assert_eq!(view.zoom, 1.25);

    let mut reopened = App::new();
    let _ = reopened.set_doc(f.map.clone(), Some(view.clone()));
    let doc = reopened.doc.as_ref().unwrap();
    assert_eq!((doc.selection().to_vec(), doc.focus()), (vec![c, b], b));
    assert_eq!(reopened.viewport.zoom(), 1.25);
    assert_eq!(reopened.viewport.scroll, iced::Vector::new(30.0, 40.0));

    // A node gone since leaves the root selected; a folded one, its shown ancestor.
    let mut gone = f.map.clone();
    gone.delete(b);
    gone.delete(c);
    let mut reopened = App::new();
    let _ = reopened.set_doc(gone, Some(view.clone()));
    assert_eq!(reopened.doc.as_ref().unwrap().focus(), root);
    let mut folded = f.map.clone();
    folded.set_folded(a, true);
    let mut reopened = App::new();
    let _ = reopened.set_doc(folded, Some(view));
    assert_eq!(reopened.doc.as_ref().unwrap().focus(), a);
}

#[test]
fn an_untitled_map_has_no_view_to_remember() {
    let app = app_with(MindMap::new("root"));
    assert!(app.current_view().is_none());
}

#[test]
fn the_window_is_restored_as_it_was_before_maximizing_or_minimizing() {
    use iced::{Point, Size};
    let mut app = app_with(MindMap::new("root"));
    send(&mut app, MapMsg::WindowResized(Size::new(900.0, 600.0)));
    send(&mut app, MapMsg::WindowMoved(Point::new(50.0, 60.0)));
    send(&mut app, MapMsg::WindowState { maximized: false, minimized: false });
    let normal = app.window;
    assert_eq!(
        (normal.size, normal.position, normal.maximized),
        (Size::new(900.0, 600.0), Some(Point::new(50.0, 60.0)), false)
    );

    // Maximized: only that is noted.
    send(&mut app, MapMsg::WindowResized(Size::new(1920.0, 1040.0)));
    send(&mut app, MapMsg::WindowMoved(Point::new(-8.0, -8.0)));
    send(&mut app, MapMsg::WindowState { maximized: true, minimized: false });
    assert_eq!(app.window, settings::WindowGeometry { maximized: true, ..normal });

    // Minimized: nothing changes.
    send(&mut app, MapMsg::WindowResized(Size::new(0.0, 0.0)));
    send(&mut app, MapMsg::WindowMoved(Point::new(-32000.0, -32000.0)));
    send(&mut app, MapMsg::WindowState { maximized: false, minimized: true });
    assert_eq!(app.window, settings::WindowGeometry { maximized: true, ..normal });
}

#[test]
fn only_edits_call_for_the_periodic_refresh() {
    let mut app = app_with(MindMap::new("root"));
    let edited = |app: &App| app.doc.as_ref().unwrap().edited_since_refresh();
    send(&mut app, MapMsg::Command(Command::ZoomIn));
    press(&mut app, Named::F5);
    send(&mut app, MapMsg::DialogOk);
    press(&mut app, Named::F2);
    send(&mut app, MapMsg::DialogCancel);
    assert!(!edited(&app));
    press(&mut app, Named::F7);
    assert!(edited(&app));
}

fn press_ctrl(app: &mut App, key: Named) {
    send(app, MapMsg::Key(key_event(Key::Named(key), Modifiers::CTRL)));
}

#[test]
fn moving_a_node_among_its_siblings_scrolls_only_to_keep_it_in_view() {
    let mut m = MindMap::new("root");
    let root = m.root();
    let kids: Vec<_> = (0..40).map(|i| m.add_child(root, &format!("node {i}"))).collect();
    let mut app = app_with(m);
    app.viewport.size = Some(iced::Size::new(800.0, 400.0));
    let rect = |app: &App, n| app.doc.as_ref().unwrap().layout().get(n).unwrap().rect;

    // Well inside the view: the map stays where it is and the node moves.
    let n = kids[20];
    app.doc.as_mut().unwrap().select(vec![n]);
    let start = iced::Vector::new(0.0, rect(&app, n).center_y() - 200.0);
    app.viewport.scroll = start;
    let before = rect(&app, n);
    press_ctrl(&mut app, Named::ArrowDown);
    assert_eq!(map(&app).index_in_parent(n), Some(21));
    assert_eq!(app.viewport.scroll, start);
    assert!(rect(&app, n).y > before.y);
    press_ctrl(&mut app, Named::ArrowUp);
    press_ctrl(&mut app, Named::ArrowUp);
    assert_eq!(map(&app).index_in_parent(n), Some(19));
    assert_eq!(app.viewport.scroll, start);

    // At the bottom edge: scrolled just enough to keep the node in view.
    let bottom = rect(&app, n).y + rect(&app, n).height;
    app.viewport.scroll = iced::Vector::new(0.0, bottom - 400.0 + 24.0);
    press_ctrl(&mut app, Named::ArrowDown);
    let r = rect(&app, n);
    assert!(r.y + r.height > bottom);
    assert_eq!(app.viewport.scroll, iced::Vector::new(0.0, r.y + r.height + 24.0 - 400.0));
}

#[test]
fn delete_and_f11_ask_first_and_ctrl_skips_the_question() {
    for key in [Named::Delete, Named::F11] {
        let mut m = MindMap::new("root");
        let root = m.root();
        let (a, b, c) = (m.add_child(root, "a"), m.add_child(root, "b"), m.add_child(root, "c"));
        let mut app = app_with(m);
        app.doc.as_mut().unwrap().select(vec![b]);
        press(&mut app, key);
        // Asked, nothing deleted yet; No (Cancel) keeps the node.
        assert!(
            matches!(app.dialogs.front(), Some(Dialog::Confirm { text, .. }) if text == "Are you sure to delete node \"b\"?")
        );
        assert_eq!(root_texts(&app), vec!["a", "b", "c"]);
        send(&mut app, MapMsg::DialogCancel);
        assert_eq!(root_texts(&app), vec!["a", "b", "c"]);
        // Yes deletes it and selects the next sibling; Undo brings it back.
        press(&mut app, key);
        send(&mut app, MapMsg::DialogOk);
        assert_eq!(root_texts(&app), vec!["a", "c"]);
        assert_eq!(app.doc.as_ref().unwrap().focus(), c);
        send(&mut app, MapMsg::Command(Command::Undo));
        assert_eq!(root_texts(&app), vec!["a", "b", "c"]);
        // Ctrl: no question.
        app.doc.as_mut().unwrap().select(vec![a, b]);
        press_ctrl(&mut app, key);
        assert!(app.dialogs.is_empty());
        assert_eq!(root_texts(&app), vec!["c"]);
        assert_eq!(app.doc.as_ref().unwrap().focus(), c);
        // The root is never deleted, and nothing is asked for it.
        app.doc.as_mut().unwrap().select(vec![root]);
        press(&mut app, key);
        press_ctrl(&mut app, key);
        assert!(app.dialogs.is_empty());
        assert_eq!(root_texts(&app), vec!["c"]);
    }
}

#[test]
fn a_failed_index_update_is_logged_and_the_status_bar_leaves_out_its_details() {
    let dir = tempfile::tempdir().unwrap();
    crate::error_log::redirect(dir.path());
    let mut app = app_with(MindMap::new("root"));
    send(&mut app, MapMsg::IndexUpdated(Err("disk full at D:\\docs\\.search-index".into())));
    assert_eq!(app.status.text(), "search index update failed (see error log)");
    let log = std::fs::read_to_string(dir.path().join(crate::error_log::FILE_NAME)).unwrap();
    assert!(log.contains("search index update failed: disk full at D:\\docs\\.search-index"), "{log}");
}

#[test]
fn a_successful_index_update_logs_nothing() {
    let dir = tempfile::tempdir().unwrap();
    crate::error_log::redirect(dir.path());
    let mut app = app_with(MindMap::new("root"));
    send(&mut app, MapMsg::IndexUpdated(Ok(())));
    assert_eq!(app.status.text(), "");
    assert!(!dir.path().join(crate::error_log::FILE_NAME).exists());
}
