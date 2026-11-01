//! The page's right-click menu: where it opens, where it leaves the cursor,
//! its commands, and the table cell's menu.

use super::*;
use crate::app::editor::RightClick;
use crate::markdown::edit::Sel;
use iced::Point;

const AT: Point = Point::new(140.0, 90.0);

/// A right click at `AT` in the window landing on `target`.
fn right_click(app: &mut App, id: window::Id, target: RightClick) {
    editor(app, id, EditorMsg::RightPressedAt(AT));
    editor(app, id, EditorMsg::RightClicked(target));
}

fn context_menu(app: &App, id: window::Id) -> Option<Point> {
    app.editors[&id].context_menu
}

fn sel(app: &App, id: window::Id) -> Sel {
    app.editors[&id].page.sel()
}

/// The pointer over the rendered text `text` of `block`, which stands for
/// `source`, at its start.
fn over_text(app: &mut App, id: window::Id, block: usize, text: &str, source: std::ops::Range<usize>) {
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
    editor(app, id, EditorMsg::PointerOver(std::sync::Arc::new(info), Point::new(0.0, 5.0)));
}

#[test]
fn a_right_click_inside_the_selection_keeps_it_for_the_menus_commands() {
    let (mut app, _f, _, _) = app_with_page("hello world\n\nOther\n");
    let id = open_with_f1(&mut app);
    select(&mut app, id, "world");
    // Shown offset 8: between `wo` and `rld`.
    right_click(&mut app, id, RightClick::Live(8));
    assert_eq!(context_menu(&app, id), Some(AT));
    assert_eq!(sel(&app, id), Sel { anchor: 6, head: 11 });
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Bold));
    assert_eq!(page_text(&app, id), "hello **world**\n\nOther\n");
    assert_eq!(context_menu(&app, id), None, "choosing an item closes the menu");
}

#[test]
fn a_right_click_outside_the_selection_moves_the_cursor_there() {
    let (mut app, _f, _, _) = app_with_page("hello world\n\nOther\n");
    let id = open_with_f1(&mut app);
    // In the same block: the cursor goes to the clicked place.
    select(&mut app, id, "world");
    right_click(&mut app, id, RightClick::Live(2));
    assert_eq!(sel(&app, id), Sel::caret(2));
    assert_eq!(context_menu(&app, id), Some(AT));
    // On another rendered block's text: into that block, where a click puts it.
    select(&mut app, id, "world");
    let other = page_text(&app, id).find("Other").unwrap();
    over_text(&mut app, id, 1, "Other", other..other + 5);
    right_click(&mut app, id, RightClick::Text);
    assert_eq!(sel(&app, id), Sel::caret(other));
    assert_eq!(active_kind(&app, id), crate::markdown::BlockKind::Paragraph);
    assert_eq!(context_menu(&app, id), Some(AT));
    // Beside a line: at its end. Nothing is being selected afterwards.
    editor(&mut app, id, EditorMsg::Released);
    right_click(&mut app, id, RightClick::At(11));
    assert_eq!(sel(&app, id), Sel::caret(11));
    assert!(!app.editors[&id].pressed(), "a right click starts no drag");
}

#[test]
fn a_right_click_on_selected_blocks_keeps_them() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    app.editors.get_mut(&id).unwrap().page.select_blocks(0, 1);
    right_click(&mut app, id, RightClick::Block(1));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((0, 1)));
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Copy));
    assert_eq!(clipboard(&app).text.as_deref(), Some("one\n\ntwo"));
    // A block outside them: the cursor goes into it.
    app.editors.get_mut(&id).unwrap().page.select_blocks(0, 1);
    right_click(&mut app, id, RightClick::Block(2));
    assert_eq!(app.editors[&id].page.selected_blocks(), None);
    assert_eq!(sel(&app, id), Sel::caret(page_text(&app, id).find("three").unwrap()));
}

#[test]
fn esc_and_a_press_beside_close_the_menu() {
    let (mut app, _f, _, _) = app_with_page("hello world\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "hello");
    right_click(&mut app, id, RightClick::Page);
    assert_eq!(sel(&app, id), Sel::caret(5), "elsewhere on the page the cursor stays");
    assert_eq!(context_menu(&app, id), Some(AT));
    editor(&mut app, id, EditorMsg::Escape);
    assert_eq!(context_menu(&app, id), None);
    assert_eq!(page_text(&app, id), "hello world\n");
    assert!(app.editors.contains_key(&id), "Esc only closed the menu");
    right_click(&mut app, id, RightClick::Page);
    editor(&mut app, id, EditorMsg::CloseContextMenu);
    assert_eq!(context_menu(&app, id), None);
    // Typing closes it too.
    right_click(&mut app, id, RightClick::Page);
    typing(&mut app, id, "!");
    assert_eq!(context_menu(&app, id), None);
}

#[test]
fn the_menu_opens_in_source_mode() {
    let (mut app, _f, _, _) = app_with_page("hello world\n");
    let id = open_with_f1(&mut app);
    editor(&mut app, id, EditorMsg::Command(EditorCommand::SourceMode));
    right_click(&mut app, id, RightClick::Editor(Point::new(5.0, 5.0)));
    assert_eq!(context_menu(&app, id), Some(AT));
    editor(&mut app, id, EditorMsg::Command(EditorCommand::SelectAll));
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Copy));
    assert_eq!(clipboard(&app).text.as_deref(), Some("hello world\n"));
}

#[test]
fn a_right_click_on_a_rendered_table_opens_the_cell_menu() {
    let (mut app, _f, _, _) = app_with_page("Intro\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "Intro");
    right_click(&mut app, id, RightClick::TableCell(1, 1, 1));
    assert_eq!(cell(&app, id), (1, 1));
    assert_eq!(app.editors[&id].table_ui.menu, Some(AT));
    assert_eq!(context_menu(&app, id), None);
    // Choosing an item closes it.
    editor(&mut app, id, EditorMsg::Command(EditorCommand::RowBelow));
    assert_eq!(app.editors[&id].page.active().unwrap().table().unwrap().table.rows.len(), 3);
    assert_eq!(app.editors[&id].table_ui.menu, None);
    // In the cell being edited: the cell's menu too.
    right_click(&mut app, id, RightClick::Live(0));
    assert_eq!(app.editors[&id].table_ui.menu, Some(AT));
}

#[test]
fn copying_a_cell_range_from_the_cell_menu() {
    let (mut app, _f, _, _) = app_with_page("| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    editor(&mut app, id, EditorMsg::CellPressed(1, 0));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Entered(2, 1)));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Released));
    editor(&mut app, id, EditorMsg::RightPressedAt(AT));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Menu(2, 0)));
    assert_eq!(app.editors[&id].table_ui.menu, Some(AT));
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Copy));
    assert_eq!(clipboard(&app).text.as_deref(), Some("1\t2\n3\t4"));
}
