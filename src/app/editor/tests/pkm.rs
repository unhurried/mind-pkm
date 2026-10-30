//! The map's PKM commands on pages open in editors: renaming, adding assets, deleting.

use super::*;

#[test]
fn renaming_an_open_page_saves_it_and_the_editor_follows() {
    let (mut app, f, _, page) = app_with_page("[assets](Plan.assets)\n\nold\n");
    std::fs::create_dir(f.doc_dir.join("Plan.assets")).unwrap();
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "old");
    typing(&mut app, id, " new");
    assert!(app.editors[&id].page.is_dirty());
    send(&mut app, Message::Key(key_event(Key::Named(Named::F2), Modifiers::empty())));
    send(&mut app, Message::DialogInput("Done".into()));
    send(&mut app, Message::DialogOk);
    let renamed = f.doc_dir.join("Done.md");
    assert!(!page.exists());
    assert_eq!(crate::markdown::page::read_page(&renamed).unwrap(), "[assets](Done.assets)\n\nold new\n");
    let editor = &app.editors[&id];
    assert_eq!(editor.page.path(), renamed);
    assert_eq!(editor.page.text(), "[assets](Done.assets)\n\nold new\n");
    assert!(!editor.page.is_dirty());
    assert_eq!(app.title(id), "Done.md - Mind PKM");
}

fn rename_with_f2(app: &mut App, name: &str) {
    send(app, Message::Key(key_event(Key::Named(Named::F2), Modifiers::empty())));
    send(app, Message::DialogInput(name.into()));
    send(app, Message::DialogOk);
}

#[test]
fn undo_in_the_map_does_not_go_back_across_a_rename() {
    let (mut app, f, node, _) = app_with_page("[assets](Plan.assets)\n");
    std::fs::create_dir(f.doc_dir.join("Plan.assets")).unwrap();
    // An edit before the rename: its undo step would bring back the old name too.
    send(&mut app, Message::Command(Command::NewChild));
    send(&mut app, Message::DialogInput("child".into()));
    send(&mut app, Message::DialogOk);
    app.doc.as_mut().unwrap().select(vec![node]);
    rename_with_f2(&mut app, "Done");
    assert_eq!(app.doc.as_ref().unwrap().map().text(node), "Done");

    send(&mut app, Message::Command(Command::Undo));
    send(&mut app, Message::Command(Command::Undo));
    let map = &app.doc.as_ref().unwrap().map();
    assert_eq!(map.text(node), "Done");
    assert!(crate::pkm::page_file(&f.doc_dir, &map.text(node)).exists());
    assert!(!f.doc_dir.join("Plan.md").exists());
    // Nothing to redo either.
    send(&mut app, Message::Command(Command::Redo));
    assert_eq!(app.doc.as_ref().unwrap().map().text(node), "Done");
}

#[test]
fn undo_in_an_editor_does_not_revert_the_references_a_rename_rewrote() {
    let (mut app, f, _, _) = app_with_page("[assets](Plan.assets)\n\nold\n");
    std::fs::create_dir(f.doc_dir.join("Plan.assets")).unwrap();
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "old");
    typing(&mut app, id, " new");
    rename_with_f2(&mut app, "Done");
    assert_eq!(page_text(&app, id), "[assets](Done.assets)\n\nold new\n");

    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "[assets](Done.assets)\n\nold new\n");
}

#[test]
fn renaming_a_directory_keeps_the_undo_of_an_open_page() {
    let mut f = Fixture::new();
    let root = f.map.root();
    let dir = f.add_dir_node(root, "Area");
    std::fs::create_dir(f.doc_dir.join("Area")).unwrap();
    let page = f.doc_dir.join("Area").join("Note.md");
    crate::markdown::page::write_page(&page, "note\n").unwrap();
    let mut app = App::new();
    let _ = app.set_doc(f.map.clone(), None);
    app.doc.as_mut().unwrap().select(vec![dir]);
    let _ = app.open_page(&page);
    let id = *app.editors.keys().last().unwrap();
    click_after(&mut app, id, "note");
    typing(&mut app, id, "!");
    rename_with_f2(&mut app, "Areas");

    // The text didn't change with the rename, so typing before it can still be undone.
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "note\n");
}

#[test]
fn pages_in_a_renamed_directory_follow_it() {
    let mut f = Fixture::new();
    let root = f.map.root();
    let dir = f.add_dir_node(root, "Area");
    std::fs::create_dir(f.doc_dir.join("Area")).unwrap();
    let page = f.doc_dir.join("Area").join("Note.md");
    crate::markdown::page::write_page(&page, "note\n").unwrap();
    let mut app = App::new();
    let _ = app.set_doc(f.map.clone(), None);
    app.doc.as_mut().unwrap().select(vec![dir]);
    let _ = app.open_page(&page);
    let id = *app.editors.keys().last().unwrap();
    click_after(&mut app, id, "note");
    typing(&mut app, id, "!");
    send(&mut app, Message::Key(key_event(Key::Named(Named::F2), Modifiers::empty())));
    send(&mut app, Message::DialogInput("Areas".into()));
    send(&mut app, Message::DialogOk);
    let moved = f.doc_dir.join("Areas").join("Note.md");
    assert_eq!(app.editors[&id].page.path(), moved);
    assert_eq!(crate::markdown::page::read_page(&moved).unwrap(), "note!\n");
}

#[test]
fn add_assets_saves_the_open_page_and_reloads_it() {
    let (mut app, f, _, page) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    typing(&mut app, id, "y");
    send(&mut app, Message::Command(Command::AddAssets));
    assert!(f.doc_dir.join("Plan.assets").is_dir());
    assert_eq!(crate::markdown::page::read_page(&page).unwrap(), "[assets](Plan.assets)\n\nxy\n");
    assert_eq!(page_text(&app, id), "[assets](Plan.assets)\n\nxy\n");
    assert!(!app.editors[&id].page.is_dirty());
}

#[test]
fn the_editors_of_deleted_pages_close() {
    // The deletion itself moves files to the Recycle Bin; here the file is
    // removed in the temporary directory, around the editor's part.
    let (mut app, _f, _, page) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    typing(&mut app, id, "y");
    app.save_editors_of(std::slice::from_ref(&page)).unwrap();
    assert_eq!(crate::markdown::page::read_page(&page).unwrap(), "xy\n");
    std::fs::remove_file(&page).unwrap();
    let _ = app.close_deleted_editors(std::slice::from_ref(&page));
    assert!(app.editors.is_empty());
}
