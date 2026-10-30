//! Find and replace.

use super::*;

fn find(app: &mut App, id: window::Id, msg: FindMsg) {
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
    find(&mut app, id, FindMsg::Next);
    assert_eq!(selected(&app, id), 21..24);
    find(&mut app, id, FindMsg::Next);
    assert_eq!(selected(&app, id), 0..3, "wraps around");
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::F3), Modifiers::SHIFT)));
    assert_eq!(selected(&app, id), 21..24);
    // Match case.
    find(&mut app, id, FindMsg::MatchCase(true));
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
    find(&mut app, id, FindMsg::Query("foo".into()));
    find(&mut app, id, FindMsg::Replacement("bar".into()));
    // Replace without a selected match goes to the first; then replaces it.
    find(&mut app, id, FindMsg::Replace);
    assert_eq!(page_text(&app, id), "foo a\n\nfoo b\n\nfoo c\n");
    find(&mut app, id, FindMsg::Replace);
    assert_eq!(page_text(&app, id), "bar a\n\nfoo b\n\nfoo c\n");
    assert_eq!(app.editors[&id].page.selected_source(), "foo", "the next match");
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "foo a\n\nfoo b\n\nfoo c\n");
    // Replace All: one change.
    find(&mut app, id, FindMsg::ReplaceAll);
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
    find(&mut app, id, FindMsg::Query("foo".into()));
    find(&mut app, id, FindMsg::Next);
    find(&mut app, id, FindMsg::Next);
    let source = app.editors[&id].source.as_ref().unwrap();
    assert_eq!(source.selection().as_deref(), Some("foo"));
    assert_eq!(crate::app::editor::page::content_sel(source, &source.text()).range(), 7..10);
    find(&mut app, id, FindMsg::Replacement("x".into()));
    find(&mut app, id, FindMsg::ReplaceAll);
    assert_eq!(page_text(&app, id), "# x\n\nx\n");
    assert_eq!(app.editors[&id].source.as_ref().unwrap().text(), "# x\n\nx\n");
}

#[test]
fn a_match_in_a_table_is_selected_in_its_cell() {
    let (mut app, _f, _, _) = app_with_page("x\n\n| a | value |\n| --- | --- |\n| 1 | 2 |\n");
    let id = open_with_f1(&mut app);
    press(&mut app, id, "f", Modifiers::CTRL);
    find(&mut app, id, FindMsg::Query("lu".into()));
    find(&mut app, id, FindMsg::Next);
    assert_eq!(cell(&app, id), (0, 1));
    assert_eq!(app.editors[&id].page.selected_source(), "lu");
    assert_eq!(app.editors[&id].find_status(), "1 of 1");
}
