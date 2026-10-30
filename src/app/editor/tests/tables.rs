//! Tables edited cell by cell.

use super::*;

const TABLE: &str = "| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n";

fn table_of(app: &App, id: window::Id) -> crate::markdown::table::Table {
    crate::markdown::table::Table::parse(&page_text(app, id)).expect("a table")
}

fn cell_range(app: &App, id: window::Id) -> Option<crate::markdown::table::CellRange> {
    app.editors[&id].page.active().unwrap().table().unwrap().range
}

fn rows(app: &App, id: window::Id) -> Vec<Vec<String>> {
    table_of(app, id).rows
}

fn named(app: &mut App, id: window::Id, key: Named, modifiers: Modifiers) {
    send(app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(key), modifiers))));
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
    let cmd = |app: &mut App, c: EditorCommand| editor(app, id, EditorMsg::Command(c));
    cmd(&mut app, EditorCommand::ColumnLeft);
    assert_eq!(rows(&app, id), strings(&[&["a", "", "b"], &["1", "", "2"], &["3", "", "4"]]));
    cmd(&mut app, EditorCommand::DeleteColumn);
    cmd(&mut app, EditorCommand::ColumnRight);
    assert_eq!(rows(&app, id), strings(&[&["a", "b", ""], &["1", "2", ""], &["3", "4", ""]]));
    assert_eq!(cell(&app, id), (1, 2));
    cmd(&mut app, EditorCommand::RowAbove);
    assert_eq!(rows(&app, id)[1], vec![String::new(); 3]);
    cmd(&mut app, EditorCommand::DeleteRow);
    cmd(&mut app, EditorCommand::DeleteColumn);
    assert_eq!(rows(&app, id), strings(&[&["a", "b"], &["1", "2"], &["3", "4"]]));
    // Delete Table.
    cmd(&mut app, EditorCommand::DeleteTable);
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
    editor(&mut app, id, EditorMsg::Command(EditorCommand::AlignCenter));
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
    editor(&mut app, id, EditorMsg::Table(TableMsg::Entered(2, 1)));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Released));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Entered(1, 1)));
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
fn clicking_a_cell_of_an_inactive_table_puts_the_cursor_in_that_cell() {
    let (mut app, _f, _, _) = app_with_page(&format!("Intro\n\n{TABLE}"));
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "Intro");
    let table = app.editors[&id].page.doc().blocks.iter().position(|b| b.kind == crate::markdown::BlockKind::Table);
    editor(&mut app, id, EditorMsg::TableCellPressed(table.unwrap(), 2, 1));
    assert_eq!(cell(&app, id), (2, 1));
    typing(&mut app, id, "Q");
    let text = page_text(&app, id);
    let table = crate::markdown::table::Table::parse(&text[text.find('|').unwrap()..]).unwrap();
    assert_eq!(table.rows, strings(&[&["a", "b"], &["1", "2"], &["3", "4Q"]]), "typed after the cell's text");
}

#[test]
fn rows_and_columns_from_the_cell_menu() {
    let (mut app, _f, _, _) = app_with_page(TABLE);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "1");
    editor(&mut app, id, EditorMsg::PointerAt(iced::Point::new(100.0, 200.0)));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Menu(2, 1)));
    assert_eq!(app.editors[&id].table_ui.menu, Some(iced::Point::new(100.0, 200.0)));
    assert_eq!(cell(&app, id), (2, 1), "the right-clicked cell");
    editor(&mut app, id, EditorMsg::Table(TableMsg::MenuItem(crate::app::editor::keys::TableMenuItem::CopyColumn)));
    assert_eq!(clipboard(&app).text.as_deref(), Some("b\n2\n4"));
    assert_eq!(app.editors[&id].table_ui.menu, None);
    editor(&mut app, id, EditorMsg::Table(TableMsg::Menu(1, 0)));
    let item = crate::app::editor::keys::TableMenuItem::Command(EditorCommand::RowBelow);
    editor(&mut app, id, EditorMsg::Table(TableMsg::MenuItem(item)));
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
    editor(&mut app, id, EditorMsg::Table(TableMsg::Rows("3".into())));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Columns("3".into())));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Resize));
    assert_eq!(rows(&app, id), strings(&[&["a", "b", ""], &["1", "2", ""], &["3", "4", ""], &["", "", ""]]));
    // Not a number: nothing changes.
    editor(&mut app, id, EditorMsg::Table(TableMsg::Rows("x".into())));
    editor(&mut app, id, EditorMsg::Table(TableMsg::Resize));
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
