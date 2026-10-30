//! The assets pane.

use super::*;

fn assets(app: &mut App, id: window::Id, msg: AssetsMsg) {
    editor(app, id, EditorMsg::Assets(msg));
}

fn pane(app: &App, id: window::Id) -> &crate::app::editor::assets::AssetsPane {
    app.editors[&id].assets.as_ref().expect("the assets pane is shown")
}

/// Finishes the read the pane asked for, as the background task would.
fn finish_read(app: &mut App, id: window::Id) {
    let dir = app.editors[&id].assets_dir();
    let tree = crate::pkm::assets::read_tree(&dir);
    assets(app, id, AssetsMsg::Read(dir, tree));
}

/// The page `Plan` with `Plan.assets/` holding `files` (a trailing `/` makes a folder).
fn app_with_assets(text: &str, files: &[&str]) -> (App, Fixture, window::Id) {
    let (mut app, f, _, _) = app_with_page(text);
    let dir = f.doc_dir.join("Plan.assets");
    std::fs::create_dir(&dir).unwrap();
    for name in files {
        match name.strip_suffix('/') {
            Some(folder) => std::fs::create_dir_all(dir.join(folder)).unwrap(),
            None => std::fs::write(dir.join(name), "x").unwrap(),
        }
    }
    let id = open_with_f1(&mut app);
    press(&mut app, id, "2", Modifiers::CTRL | Modifiers::SHIFT);
    finish_read(&mut app, id);
    (app, f, id)
}

fn asset(f: &Fixture, name: &str) -> PathBuf {
    f.doc_dir.join("Plan.assets").join(name)
}

/// What an editor asks of the application for a message (without the application doing it).
fn effects_of(app: &mut App, id: window::Id, msg: EditorMsg) -> Vec<Effect> {
    let prefs = app.editor_prefs.clone();
    let mut effects = Vec::new();
    let _ = app.editors.get_mut(&id).unwrap().update(msg, &prefs, &mut effects);
    effects
}

#[test]
fn the_assets_pane_is_shown_per_window_and_hidden_when_a_window_opens() {
    let (mut app, f, id) = app_with_assets("x\n", &[]);
    assert!(app.editors[&id].assets.is_some());
    // Another page opens without it; the first keeps it.
    let other = f.write_page("Other", "y\n");
    let _ = app.open_in_editor(&other);
    let other_id = *app.editors.iter().find(|(_, e)| e.page.path() == other).unwrap().0;
    assert!(app.editors[&other_id].assets.is_none());
    assert!(app.editors[&id].assets.is_some());
    // The View menu entry toggles it too.
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Assets));
    assert!(app.editors[&id].assets.is_none());
}

#[test]
fn the_assets_pane_lists_the_folder_as_a_tree() {
    let (mut app, f, id) = app_with_assets("x\n", &["map.png", "Notes.txt", "raw/", "raw/a.txt", "Bin/"]);
    assert_eq!(pane(&app, id).shown().unwrap(), vec!["▸ Bin", "▸ raw", "map.png", "Notes.txt"]);
    assets(&mut app, id, AssetsMsg::Toggle(asset(&f, "raw")));
    assert_eq!(pane(&app, id).shown().unwrap(), vec!["▸ Bin", "▾ raw", "  a.txt", "map.png", "Notes.txt"]);
    // A refresh keeps it expanded, and lists what was added meanwhile.
    std::fs::write(asset(&f, "raw/b.txt"), "").unwrap();
    assets(&mut app, id, AssetsMsg::Refresh);
    assert!(pane(&app, id).reading());
    finish_read(&mut app, id);
    assert_eq!(pane(&app, id).shown().unwrap(), vec!["▸ Bin", "▾ raw", "  a.txt", "  b.txt", "map.png", "Notes.txt"]);
    // Double-clicking a folder expands and collapses it too.
    assets(&mut app, id, AssetsMsg::DoubleClicked(asset(&f, "raw")));
    assert_eq!(pane(&app, id).shown().unwrap()[1], "▸ raw");
    // A click highlights.
    assets(&mut app, id, AssetsMsg::Pressed(asset(&f, "map.png")));
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(pane(&app, id).selected(), Some(asset(&f, "map.png").as_path()));
}

#[test]
fn a_page_without_an_assets_folder() {
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    press(&mut app, id, "2", Modifiers::CTRL | Modifiers::SHIFT);
    finish_read(&mut app, id);
    assert!(pane(&app, id).has_no_folder());
}

#[test]
fn the_assets_pane_reads_the_folder_again() {
    let (mut app, f, id) = app_with_assets("x\n", &["a.png"]);
    assert!(!pane(&app, id).reading());
    // The window activated.
    send(&mut app, Message::Focused(id));
    assert!(pane(&app, id).reading());
    finish_read(&mut app, id);
    // Files inserted.
    editor(&mut app, id, EditorMsg::Inserted(Ok(vec!["![](Plan.assets/b.png)".into()])));
    assert!(pane(&app, id).reading());
    finish_read(&mut app, id);
    // The page renamed (F2): the pane shows the renamed folder.
    send(&mut app, Message::Key(key_event(Key::Named(Named::F2), Modifiers::empty())));
    send(&mut app, Message::DialogInput("Done".into()));
    send(&mut app, Message::DialogOk);
    assert!(pane(&app, id).reading());
    assert_eq!(app.editors[&id].assets_dir(), f.doc_dir.join("Done.assets"));
    finish_read(&mut app, id);
    assert_eq!(pane(&app, id).shown().unwrap(), vec!["a.png"]);
    // A read of the folder before the rename is not taken.
    assets(&mut app, id, AssetsMsg::Read(f.doc_dir.join("Plan.assets"), None));
    assert_eq!(pane(&app, id).shown().unwrap(), vec!["a.png"]);
}

#[test]
fn double_clicking_an_asset_opens_it() {
    let (mut app, f, id) = app_with_assets("x\n", &["report.pdf", "notes.md"]);
    let pdf = asset(&f, "report.pdf");
    let effects = effects_of(&mut app, id, EditorMsg::Assets(AssetsMsg::DoubleClicked(pdf.clone())));
    assert_eq!(effects, vec![Effect::OpenExternal(pdf.display().to_string())]);
    let md = asset(&f, "notes.md");
    let effects = effects_of(&mut app, id, EditorMsg::Assets(AssetsMsg::DoubleClicked(md.clone())));
    assert_eq!(effects, vec![Effect::OpenPage(md)]);
}

/// Right-clicks an item and chooses Rename, typing `name`.
fn rename_asset(app: &mut App, id: window::Id, item: &std::path::Path, name: &str) {
    assets(app, id, AssetsMsg::RightClicked(item.to_path_buf()));
    assets(app, id, AssetsMsg::Rename);
    assets(app, id, AssetsMsg::NewName(name.into()));
    editor(app, id, EditorMsg::Dialog(DialogAnswer::Ok));
}

#[test]
fn renaming_an_asset_rewrites_the_page_links() {
    let text = "![](Plan.assets/map.png)\n\n[raw](./Plan.assets/raw/a%20b.txt)\n";
    let (mut app, f, id) = app_with_assets(text, &["map.png", "b.png", "raw/", "raw/a b.txt"]);
    assets(&mut app, id, AssetsMsg::RightClicked(asset(&f, "map.png")));
    let menu = pane(&app, id).menu.clone().unwrap();
    assert!(menu.can_change);
    assets(&mut app, id, AssetsMsg::Rename);
    assert_eq!(
        app.editors[&id].dialogs.front(),
        Some(&EditorDialog::RenameAsset { path: asset(&f, "map.png"), name: "map.png".into() })
    );
    assets(&mut app, id, AssetsMsg::NewName("route.png".into()));
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Ok));
    assert!(asset(&f, "route.png").is_file() && !asset(&f, "map.png").exists());
    assert_eq!(page_text(&app, id), "![](Plan.assets/route.png)\n\n[raw](./Plan.assets/raw/a%20b.txt)\n");
    assert!(app.editors[&id].page.is_dirty());
    assert!(pane(&app, id).reading());
    finish_read(&mut app, id);
    // A folder, with the links into it.
    rename_asset(&mut app, id, &asset(&f, "raw"), "source");
    assert_eq!(page_text(&app, id), "![](Plan.assets/route.png)\n\n[raw](./Plan.assets/source/a%20b.txt)\n");
    // Undo takes back the page's text, one rename at a time.
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "![](Plan.assets/route.png)\n\n[raw](./Plan.assets/raw/a%20b.txt)\n");
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), text);
    // A taken name.
    rename_asset(&mut app, id, &asset(&f, "route.png"), "b.png");
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Message("b.png already exists.".into())));
    assert!(asset(&f, "route.png").is_file());
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Ok));
    // Cancelled, and the same name: nothing.
    assets(&mut app, id, AssetsMsg::RightClicked(asset(&f, "route.png")));
    assets(&mut app, id, AssetsMsg::Rename);
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Cancel));
    rename_asset(&mut app, id, &asset(&f, "route.png"), "route.png");
    assert!(app.editors[&id].dialogs.is_empty() && asset(&f, "route.png").is_file());
}

#[test]
fn deleting_an_asset_asks_first() {
    let (mut app, f, id) = app_with_assets("x\n", &["old.png", "raw/", "raw/a.txt"]);
    assets(&mut app, id, AssetsMsg::RightClicked(asset(&f, "old.png")));
    assets(&mut app, id, AssetsMsg::Delete);
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::DeleteAsset(asset(&f, "old.png"))));
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Cancel));
    assert!(asset(&f, "old.png").exists());
    assets(&mut app, id, AssetsMsg::RightClicked(asset(&f, "old.png")));
    assets(&mut app, id, AssetsMsg::Delete);
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Ok));
    assert!(!asset(&f, "old.png").exists());
    finish_read(&mut app, id);
    assert_eq!(pane(&app, id).shown().unwrap(), vec!["▸ raw"]);
    // A folder with what it holds.
    assets(&mut app, id, AssetsMsg::RightClicked(asset(&f, "raw")));
    assets(&mut app, id, AssetsMsg::Delete);
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Ok));
    assert!(!asset(&f, "raw").exists() && f.doc_dir.join("Plan.assets").is_dir());
}

#[test]
fn assets_outside_the_document_directory_or_open_are_not_changed() {
    // An item holding a file open in an editor window.
    let (mut app, f, id) = app_with_assets("x\n", &["notes.md"]);
    let _ = app.open_in_editor(&asset(&f, "notes.md"));
    assets(&mut app, id, AssetsMsg::RightClicked(asset(&f, "notes.md")));
    assert!(!pane(&app, id).menu.as_ref().unwrap().can_change);
    // Chosen anyway: nothing happens.
    assets(&mut app, id, AssetsMsg::Rename);
    assets(&mut app, id, AssetsMsg::Delete);
    assert!(app.editors[&id].dialogs.is_empty());
    // Even asked for directly, the application refuses.
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Ok));
    let editor_window = app.editors.get_mut(&id).unwrap();
    editor_window.dialogs.push_back(EditorDialog::DeleteAsset(asset(&f, "notes.md")));
    editor(&mut app, id, EditorMsg::Dialog(DialogAnswer::Ok));
    assert!(asset(&f, "notes.md").exists());
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Message("notes.md is open in an editor.".into())));
    // A file outside the document directory.
    let outside = f.dir.path().join("Loose.md");
    std::fs::write(&outside, "z\n").unwrap();
    std::fs::create_dir(f.dir.path().join("Loose.assets")).unwrap();
    let kept = f.dir.path().join("Loose.assets").join("keep.png");
    std::fs::write(&kept, "").unwrap();
    let _ = app.open_in_editor(&outside);
    let loose = *app.editors.iter().find(|(_, e)| e.page.path() == outside).unwrap().0;
    press(&mut app, loose, "2", Modifiers::CTRL | Modifiers::SHIFT);
    finish_read(&mut app, loose);
    assert_eq!(pane(&app, loose).shown().unwrap(), vec!["keep.png"]);
    assets(&mut app, loose, AssetsMsg::RightClicked(kept.clone()));
    assert!(!pane(&app, loose).menu.as_ref().unwrap().can_change);
    app.editors.get_mut(&loose).unwrap().dialogs.push_back(EditorDialog::DeleteAsset(kept.clone()));
    editor(&mut app, loose, EditorMsg::Dialog(DialogAnswer::Ok));
    assert!(kept.exists());
    app.editors.get_mut(&loose).unwrap().dialogs.clear();
    app.editors
        .get_mut(&loose)
        .unwrap()
        .dialogs
        .push_back(EditorDialog::RenameAsset { path: kept.clone(), name: "moved.png".into() });
    editor(&mut app, loose, EditorMsg::Dialog(DialogAnswer::Ok));
    assert!(kept.exists());
}

/// Presses an item of the pane and moves the pointer far enough to drag it.
fn start_drag(app: &mut App, id: window::Id, item: &std::path::Path) {
    editor(app, id, EditorMsg::PointerAt(iced::Point::new(800.0, 100.0)));
    assets(app, id, AssetsMsg::Pressed(item.to_path_buf()));
    editor(app, id, EditorMsg::PointerAt(iced::Point::new(780.0, 104.0)));
    editor(app, id, EditorMsg::PointerAt(iced::Point::new(400.0, 120.0)));
    assert!(app.editors[&id].asset_dragging());
}

/// The pointer over rendered text of `block` standing for `source`, at `x` in it.
fn over_text_at(app: &mut App, id: window::Id, block: usize, text: &str, source: std::ops::Range<usize>, x: f32) {
    let info = crate::app::editor::view::HitInfo {
        block,
        runs: vec![crate::app::editor::view::HitRun {
            text: text.into(),
            size: crate::app::editor::view::BODY_SIZE,
            font: crate::app::view::APP_FONT,
            map: Some(crate::markdown::content::source_map(text.len(), source)),
        }],
        width: 600.0,
    };
    editor(app, id, EditorMsg::PointerOver(std::sync::Arc::new(info), iced::Point::new(x, 5.0)));
}

/// The pointer over rendered text of `block` standing for `source`, at its start.
fn over_text(app: &mut App, id: window::Id, block: usize, text: &str, source: std::ops::Range<usize>) {
    over_text_at(app, id, block, text, source, 0.0);
}

#[test]
fn an_asset_dragged_into_text_goes_where_it_is_dropped() {
    let (mut app, f, id) = app_with_assets("see here\n", &["map.png"]);
    start_drag(&mut app, id, &asset(&f, "map.png"));
    over_text(&mut app, id, 0, "here", 4..8);
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    assets(&mut app, id, AssetsMsg::PageHovered);
    assert!(matches!(app.editors[&id].drop_target(), Some(crate::app::editor::assets::DropTarget::Text { at: 4, .. })));
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "see ![](Plan.assets/map.png)here\n");
    assert!(!app.editors[&id].asset_pressed());
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "see here\n");
}

#[test]
fn an_asset_dropped_beside_a_block_follows_it() {
    let (mut app, f, id) = app_with_assets("Summary\n\nNext\n", &["report 1.pdf"]);
    start_drag(&mut app, id, &asset(&f, "report 1.pdf"));
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    assets(&mut app, id, AssetsMsg::PageHovered);
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "Summary\n\n[report 1.pdf](Plan.assets/report 1.pdf)\n\nNext\n");
    // Right of the text of a paragraph (its rendered text spans the column): beside it.
    let (mut app, f, id) = app_with_assets("Summary\n", &["a.txt"]);
    start_drag(&mut app, id, &asset(&f, "a.txt"));
    over_text_at(&mut app, id, 0, "Summary", 0..7, 500.0);
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    assets(&mut app, id, AssetsMsg::PageHovered);
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "Summary\n\n[a.txt](Plan.assets/a.txt)\n");
    // On a code block: after it, never into its code.
    let (mut app, f, id) = app_with_assets("```\ncode\n```\n", &["a.txt"]);
    start_drag(&mut app, id, &asset(&f, "a.txt"));
    over_text(&mut app, id, 0, "code", 4..8);
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    assets(&mut app, id, AssetsMsg::PageHovered);
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "```\ncode\n```\n\n[a.txt](Plan.assets/a.txt)\n");
    // In a list item: at the end of its text.
    let (mut app, f, id) = app_with_assets("- one\n- two\n", &["a.txt"]);
    start_drag(&mut app, id, &asset(&f, "a.txt"));
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "- one [a.txt](Plan.assets/a.txt)\n- two\n");
}

#[test]
fn dragging_an_asset_leaves_the_active_block() {
    // The page is all rendered: nothing goes into the Markdown the block shows at its cursor.
    let (mut app, f, id) = app_with_assets("see ![](Plan.assets/map.png)\n", &["map.png", "a.txt"]);
    click_after(&mut app, id, "see ");
    assert!(app.editors[&id].page.active().is_some());
    start_drag(&mut app, id, &asset(&f, "a.txt"));
    assert!(app.editors[&id].page.active().is_none());
}

#[test]
fn a_folder_dropped_on_an_empty_page() {
    let (mut app, f, id) = app_with_assets("", &["raw/"]);
    start_drag(&mut app, id, &asset(&f, "raw"));
    assets(&mut app, id, AssetsMsg::GapHovered(0));
    assets(&mut app, id, AssetsMsg::PageHovered);
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "[raw](Plan.assets/raw)");
}

#[test]
fn an_asset_released_off_the_page_inserts_nothing() {
    let (mut app, f, id) = app_with_assets("Summary\n", &["a.png"]);
    start_drag(&mut app, id, &asset(&f, "a.png"));
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    assets(&mut app, id, AssetsMsg::PageHovered);
    assets(&mut app, id, AssetsMsg::LeftPage);
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "Summary\n");
    // Beside the page's column (over the page, but no block).
    start_drag(&mut app, id, &asset(&f, "a.png"));
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    assets(&mut app, id, AssetsMsg::PageHovered);
    assets(&mut app, id, AssetsMsg::PageHovered);
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "Summary\n");
    // A press without a drag is a click.
    assets(&mut app, id, AssetsMsg::Pressed(asset(&f, "a.png")));
    assets(&mut app, id, AssetsMsg::BlockHovered(0));
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "Summary\n");
}

#[test]
fn an_asset_dropped_in_source_mode_goes_at_the_cursor() {
    let (mut app, f, id) = app_with_assets("ab\n", &["a.png"]);
    click_after(&mut app, id, "a");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::SourceMode));
    start_drag(&mut app, id, &asset(&f, "a.png"));
    assets(&mut app, id, AssetsMsg::PageHovered);
    editor(&mut app, id, EditorMsg::Released);
    assert_eq!(page_text(&app, id), "a![](Plan.assets/a.png)b\n");
}
