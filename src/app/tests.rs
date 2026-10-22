//! The application driven by its messages, without a window: what the user
//! does (keys, commands, dialog answers) and what arrives from the background.

use super::command::Command;
use super::dialog::Dialog;
use super::file::AfterUnsaved;
use super::{App, Message, settings};
use crate::mindmap::MindMap;
use crate::pkm::filter::Filter;
use crate::pkm::next_steps;
use crate::pkm::testutil::Fixture;
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers, key::Named};

fn app_with(map: MindMap) -> App {
    let mut app = App::new();
    let _ = app.set_doc(map, None);
    app
}

fn send(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn press(app: &mut App, key: Named) {
    send(
        app,
        Message::Key(keyboard::Event::KeyPressed {
            key: Key::Named(key),
            modified_key: Key::Named(key),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        }),
    );
}

fn map(app: &App) -> &MindMap {
    &app.doc.as_ref().unwrap().map
}

fn root_texts(app: &App) -> Vec<String> {
    map(app).child_texts(map(app).root())
}

#[test]
fn a_new_node_asks_for_its_text_and_undo_removes_it() {
    let mut app = app_with(MindMap::new("root"));
    press(&mut app, Named::Tab);
    assert!(matches!(app.dialogs.first(), Some(Dialog::Input { .. })));
    send(&mut app, Message::DialogInput("child".into()));
    press(&mut app, Named::Enter);
    assert!(app.dialogs.is_empty());
    assert_eq!(root_texts(&app), vec!["child"]);
    assert!(app.doc.as_ref().unwrap().dirty);
    send(&mut app, Message::Command(Command::Undo));
    assert!(root_texts(&app).is_empty());
}

#[test]
fn cancelling_a_new_node_leaves_the_map_as_it_was() {
    let mut app = app_with(MindMap::new("root"));
    let before = map(&app).to_xml();
    press(&mut app, Named::Tab);
    send(&mut app, Message::Escape);
    assert!(app.dialogs.is_empty());
    assert_eq!(map(&app).to_xml(), before);
    // Leaving the text empty removes it too.
    press(&mut app, Named::Enter);
    send(&mut app, Message::DialogOk);
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
    let generation = app.doc.as_ref().unwrap().generation;
    send(&mut app, Message::NextStepsComputed(generation, Box::new(update)));
    send(&mut app, Message::Escape);
    assert_eq!(map(&app).child_texts(page), vec!["One"]);
    assert_eq!(root_texts(&app), vec!["config", "ToDo", "Task"]);
    // Undo goes back to before the refresh, without the new node.
    send(&mut app, Message::Command(Command::Undo));
    assert!(map(&app).child_texts(page).is_empty());
    assert_eq!(root_texts(&app), vec!["config", "ToDo", "Task"]);
}

fn edited_app() -> App {
    let mut app = app_with(MindMap::new("root"));
    press(&mut app, Named::Tab);
    send(&mut app, Message::DialogInput("x".into()));
    send(&mut app, Message::DialogOk);
    app
}

#[test]
fn closing_with_unsaved_changes_asks_and_cancel_keeps_them() {
    let mut app = edited_app();
    send(&mut app, Message::CloseRequested);
    assert!(matches!(app.dialogs.first(), Some(Dialog::Unsaved(AfterUnsaved::Exit))));
    // Enter doesn't answer this question; Cancel and Esc keep the map.
    press(&mut app, Named::Enter);
    assert_eq!(app.dialogs.len(), 1);
    send(&mut app, Message::DialogCancel);
    assert!(app.dialogs.is_empty());
    assert_eq!(root_texts(&app), vec!["x"]);
    assert!(app.doc.as_ref().unwrap().dirty);
    send(&mut app, Message::CloseRequested);
    send(&mut app, Message::Escape);
    assert!(app.dialogs.is_empty());
    assert_eq!(root_texts(&app), vec!["x"]);
}

#[test]
fn new_map_with_unsaved_changes_can_discard_them() {
    let mut app = edited_app();
    send(&mut app, Message::Command(Command::New));
    assert!(matches!(app.dialogs.first(), Some(Dialog::Unsaved(AfterUnsaved::New))));
    send(&mut app, Message::DialogDiscard);
    let doc = app.doc.as_ref().unwrap();
    assert!(!doc.dirty && doc.map.path().is_none());
    assert_eq!(doc.map.text(doc.map.root()), "Map");
    assert_eq!(root_texts(&app), vec!["config", "ToDo"]);
}

#[test]
fn the_filter_dialog_sets_the_filter_and_cancel_keeps_it() {
    let mut app = app_with(MindMap::new("root"));
    assert_eq!(app.doc.as_ref().unwrap().filter, Filter::HideArchive);
    press(&mut app, Named::F5);
    send(&mut app, Message::DialogInput("t".into()));
    send(&mut app, Message::DialogOk);
    assert_eq!(app.doc.as_ref().unwrap().filter, Filter::ToDo);
    press(&mut app, Named::F5);
    send(&mut app, Message::Escape);
    assert_eq!(app.doc.as_ref().unwrap().filter, Filter::ToDo);
    assert!(app.dialogs.is_empty());
}

#[test]
fn typing_in_the_search_box_reaches_the_map_only_through_function_keys() {
    let mut app = app_with(MindMap::new("root"));
    app.search.open = true;
    app.search.input_focused = true;
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

#[test]
fn folding_by_click_is_one_undo_step() {
    let mut m = MindMap::new("root");
    let root = m.root();
    let a = m.add_child(root, "a");
    m.add_child(a, "a1");
    let mut app = app_with(m);
    send(&mut app, Message::FoldClicked(a));
    assert!(map(&app).is_folded(a));
    assert!(!app.doc.as_ref().unwrap().layout.contains(map(&app).children(a)[0]));
    send(&mut app, Message::Command(Command::Undo));
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
    assert_eq!(app.doc.as_ref().unwrap().focus, b1);
    press(&mut app, Named::ArrowDown);
    assert_eq!(app.doc.as_ref().unwrap().focus, b1);
    press(&mut app, Named::ArrowUp);
    assert_eq!(app.doc.as_ref().unwrap().focus, a2);
    press(&mut app, Named::ArrowUp);
    assert_eq!(app.doc.as_ref().unwrap().focus, a1);
    press(&mut app, Named::ArrowUp);
    assert_eq!(app.doc.as_ref().unwrap().focus, a1);
}

#[test]
fn a_map_opens_as_it_was_last_shown() {
    let mut f = Fixture::new();
    let root = f.map.root();
    let a = f.add_child(root, "a");
    let b = f.add_child(a, "b");
    let c = f.add_child(root, "c");
    let mut app = app_with(f.map.clone());
    send(&mut app, Message::Command(Command::ZoomIn));
    send(&mut app, Message::Command(Command::ZoomIn));
    app.doc.as_mut().unwrap().select(vec![c, b]);
    app.scroll_offset = iced::Vector::new(30.0, 40.0);
    let (path, view) = app.current_view().unwrap();
    assert_eq!(path, f.dir.path().join("map.mm"));
    assert_eq!(view.zoom, 1.25);

    let mut reopened = App::new();
    let _ = reopened.set_doc(f.map.clone(), Some(view.clone()));
    let doc = reopened.doc.as_ref().unwrap();
    assert_eq!((doc.selection.clone(), doc.focus), (vec![c, b], b));
    assert_eq!(reopened.zoom, 1.25);
    assert_eq!(reopened.scroll_offset, iced::Vector::new(30.0, 40.0));

    // A node gone since leaves the root selected; a folded one, its shown ancestor.
    let mut gone = f.map.clone();
    gone.delete(b);
    gone.delete(c);
    let mut reopened = App::new();
    let _ = reopened.set_doc(gone, Some(view.clone()));
    assert_eq!(reopened.doc.as_ref().unwrap().focus, root);
    let mut folded = f.map.clone();
    folded.set_folded(a, true);
    let mut reopened = App::new();
    let _ = reopened.set_doc(folded, Some(view));
    assert_eq!(reopened.doc.as_ref().unwrap().focus, a);
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
    send(&mut app, Message::WindowResized(Size::new(900.0, 600.0)));
    send(&mut app, Message::WindowMoved(Point::new(50.0, 60.0)));
    send(&mut app, Message::WindowState { maximized: false, minimized: false });
    let normal = app.window;
    assert_eq!(
        (normal.size, normal.position, normal.maximized),
        (Size::new(900.0, 600.0), Some(Point::new(50.0, 60.0)), false)
    );

    // Maximized: only that is noted.
    send(&mut app, Message::WindowResized(Size::new(1920.0, 1040.0)));
    send(&mut app, Message::WindowMoved(Point::new(-8.0, -8.0)));
    send(&mut app, Message::WindowState { maximized: true, minimized: false });
    assert_eq!(app.window, settings::WindowGeometry { maximized: true, ..normal });

    // Minimized: nothing changes.
    send(&mut app, Message::WindowResized(Size::new(0.0, 0.0)));
    send(&mut app, Message::WindowMoved(Point::new(-32000.0, -32000.0)));
    send(&mut app, Message::WindowState { maximized: false, minimized: true });
    assert_eq!(app.window, settings::WindowGeometry { maximized: true, ..normal });
}

#[test]
fn only_edits_call_for_the_periodic_refresh() {
    let mut app = app_with(MindMap::new("root"));
    let edited = |app: &App| app.doc.as_ref().unwrap().edited_since_refresh;
    send(&mut app, Message::Command(Command::ZoomIn));
    press(&mut app, Named::F5);
    send(&mut app, Message::DialogOk);
    press(&mut app, Named::F2);
    send(&mut app, Message::DialogCancel);
    assert!(!edited(&app));
    press(&mut app, Named::F7);
    assert!(edited(&app));
}
