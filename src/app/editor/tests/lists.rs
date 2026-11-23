//! Editing lists through the page editor, including keyboard indentation
//! and the blank lines of lists changed by an edit.

use super::*;

/// Runs a command of the editor's menus.
fn command(app: &mut App, id: window::Id, c: EditorCommand) {
    editor(app, id, EditorMsg::Command(c));
}

#[test]
fn enter_writes_a_mixed_list_in_one_form() {
    let (mut app, _f, _, _) = app_with_page("* a\n\n* b\n* c\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "c");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "d");
    assert_eq!(page_text(&app, id), "* a\n* b\n* c\n* d\n");
    // Undo takes back the new item and the blank lines together.
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "* a\n\n* b\n* c\n");
}

#[test]
fn an_item_into_the_item_before_makes_the_list_loose() {
    let (mut app, _f, _, _) = app_with_page("- a\n- b\n- c\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "- a\n- ");
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "- a\n\n  b\n\n- c\n");
}

#[test]
fn a_paragraph_typed_under_an_item_makes_the_list_loose_and_back() {
    let (mut app, _f, _, _) = app_with_page("* a\n* b\n* c\n\nend\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "b");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    type_keys(&mut app, id, "more");
    // Leaving the paragraph writes the whole list loose; the click lands where it was aimed.
    click_after(&mut app, id, "en");
    assert_eq!(page_text(&app, id), "* a\n\n* b\n\n  more\n\n* c\n\nend\n");
    assert_eq!(app.editors[&id].page.region_text(), Some("end"));
    assert_eq!(app.editors[&id].page.sel().head, page_text(&app, id).find("en").unwrap() + 2);
    // The list is shown loose: every item has room around it.
    let doc = crate::markdown::parse::Document::parse(&page_text(&app, id));
    assert!(matches!(&doc.blocks[0].kind, crate::markdown::BlockKind::ListItem(i) if i.loose));
    // Deleting the paragraph's text and leaving it: tight again.
    click_after(&mut app, id, "more");
    for _ in 0..4 {
        editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    }
    click_after(&mut app, id, "en");
    assert_eq!(page_text(&app, id), "* a\n* b\n* c\n\nend\n");
    assert_eq!(app.editors[&id].page.region_text(), Some("end"));
}

#[test]
fn a_code_block_typed_under_an_item_keeps_the_list_tight() {
    let (mut app, _f, _, _) = app_with_page("* a\n* b\n* c\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "b");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    type_keys(&mut app, id, "```");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "x");
    assert_eq!(page_text(&app, id), "* a\n* b\n  ```\n  x\n  ```\n* c\n");
    click_after(&mut app, id, "* a");
    assert_eq!(page_text(&app, id), "* a\n* b\n  ```\n  x\n  ```\n* c\n");
    let doc = crate::markdown::parse::Document::parse(&page_text(&app, id));
    assert!(matches!(&doc.blocks[0].kind, crate::markdown::BlockKind::ListItem(i) if !i.loose));
}

#[test]
fn blocks_inserted_in_an_item_stay_in_its_list() {
    // A code block: in the item, the list tight.
    let (mut app, _f, _, _) = app_with_page("* a\n* b\n* c\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "b");
    command(&mut app, id, EditorCommand::CodeBlock);
    assert_eq!(page_text(&app, id), "* a\n* b\n  ```\n\n  ```\n* c\n");
    assert!(matches!(active_kind(&app, id), crate::markdown::BlockKind::Code { .. }));
    typing(&mut app, id, "x");
    assert_eq!(page_text(&app, id), "* a\n* b\n  ```\n  x\n  ```\n* c\n");
    // A table in an ordered list: one list, its numbers kept.
    let (mut app, _f, _, _) = app_with_page("1. a\n2. b\n3. c\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "b");
    command(&mut app, id, EditorCommand::InsertTable);
    let text = page_text(&app, id);
    assert_eq!(text, "1. a\n2. b\n   |  |  |\n   | --- | --- |\n   |  |  |\n   |  |  |\n3. c\n");
    let doc = crate::markdown::parse::Document::parse(&text);
    let numbers: Vec<Option<u64>> = doc
        .blocks
        .iter()
        .filter_map(|b| match &b.kind {
            crate::markdown::BlockKind::ListItem(i) => Some(i.number),
            _ => None,
        })
        .collect();
    assert_eq!(numbers, [Some(1), Some(2), Some(3)]);
    assert_eq!(doc.lists.len(), 1);
    // A rule needs a blank line before it (else `b` would become a heading): the list is loose.
    let (mut app, _f, _, _) = app_with_page("* a\n* b\n* c\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "b");
    command(&mut app, id, EditorCommand::HorizontalRule);
    assert_eq!(page_text(&app, id), "* a\n\n* b\n\n  ---\n\n* c\n");
    // An item with a nested list: the block goes between its text and the nested list.
    let (mut app, _f, _, _) = app_with_page("* a\n  * n\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "a");
    command(&mut app, id, EditorCommand::CodeBlock);
    assert_eq!(page_text(&app, id), "* a\n  ```\n\n  ```\n  * n\n");
}

#[test]
fn a_list_nobody_edited_keeps_its_source() {
    let (mut app, _f, _, _) = app_with_page("* a\n\n* b\n* c\n\npara\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "para");
    typing(&mut app, id, "!");
    click_after(&mut app, id, "* a");
    // Entering and leaving an item without typing changes nothing either.
    click_after(&mut app, id, "para!");
    assert_eq!(page_text(&app, id), "* a\n\n* b\n* c\n\npara!\n");
}

#[test]
fn replacing_and_undo_do_not_tidy() {
    let (mut app, _f, _, _) = app_with_page("* a\n\n* b\n* c\n");
    let id = open_with_f1(&mut app);
    // Find and replace writes the text as it is.
    let page = &mut app.editors.get_mut(&id).unwrap().page;
    page.set_text("* a\n\n* x\n* c\n".into(), crate::markdown::edit::Sel::caret(0));
    assert_eq!(page_text(&app, id), "* a\n\n* x\n* c\n");
    click_after(&mut app, id, "c");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    assert_eq!(page_text(&app, id), "* a\n* x\n* c\n* \n");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "* a\n\n* x\n* c\n");
    click_after(&mut app, id, "* a");
    assert_eq!(page_text(&app, id), "* a\n\n* x\n* c\n");
}

#[test]
fn a_press_read_before_blocks_were_selected_lands_where_aimed() {
    // A press can come after the blocks under a still-held button were
    // selected, with an offset the view read before: the lists are tidied
    // only then, so that the offset still holds.
    let (mut app, _f, _, _) = app_with_page("* a\n* b\n* c\n\n1. one\n2. two\n\nend\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "c");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    type_keys(&mut app, id, "more");
    let end = page_text(&app, id).find("end").unwrap();
    let last = app.editors[&id].page.doc().blocks.len() - 1;
    app.editors.get_mut(&id).unwrap().page.select_blocks(last, last);
    assert_eq!(page_text(&app, id), "* a\n* b\n* c\n\n  more\n\n1. one\n2. two\n\nend\n");
    editor(&mut app, id, EditorMsg::PressedAt(end + 1));
    assert_eq!(page_text(&app, id), "* a\n\n* b\n\n* c\n\n  more\n\n1. one\n2. two\n\nend\n");
    assert_eq!(app.editors[&id].page.region_text(), Some("end"));
    assert_eq!(app.editors[&id].page.sel().head, page_text(&app, id).find("end").unwrap() + 1);
}

#[test]
fn typing_then_a_rule_that_changes_nothing_still_tidies() {
    let (mut app, _f, _, _) = app_with_page("* a\n\n* b\n\nend\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "a");
    typing(&mut app, id, "!");
    // Tab on the first item has nothing to do.
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(page_text(&app, id), "* a!\n* b\n\nend\n");
    // Typing, then the text replaced as it is (find and replace): nothing is tidied later.
    let (mut app, _f, _, _) = app_with_page("* a\n\n* b\n\nend\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "en");
    typing(&mut app, id, "!");
    let text = page_text(&app, id);
    app.editors.get_mut(&id).unwrap().page.set_text(text, crate::markdown::edit::Sel::caret(0));
    click_after(&mut app, id, "* a");
    click_after(&mut app, id, "en!");
    assert_eq!(page_text(&app, id), "* a\n\n* b\n\nen!d\n");
}

#[test]
fn backspace_in_an_empty_task_item_makes_a_paragraph_at_once() {
    let (mut app, _f, _, _) = app_with_page("- [ ] a\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "a");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    assert_eq!(page_text(&app, id), "- [ ] a\n- [ ] \n");
    // No empty plain item between: the box and the bullet go together.
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "- [ ] a\n");
    type_keys(&mut app, id, "more");
    assert_eq!(page_text(&app, id), "- [ ] a\n\n  more\n");
    // Undo takes back the typing, then the Backspace: the empty task item again.
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "- [ ] a\n- [ ] \n");
}

#[test]
fn tab_at_a_parent_indents_its_first_child() {
    let original = "* aaa\n* bbb\n  * ccc\n  * ddd\n";
    let (mut app, _f, _, _) = app_with_page(original);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bb");
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(page_text(&app, id), "* aaa\n* bbb\n  * ddd\n    * ccc\n");
    let page = &app.editors[&id].page;
    assert_eq!(page.sel().head, page.text().find("bbb").unwrap() + 2);
    assert_eq!(page.region_text(), Some("* bbb"));

    command(&mut app, id, EditorCommand::Undo);
    assert_eq!(page_text(&app, id), original);
}

#[test]
fn tab_indents_a_selected_parent_and_child_without_the_following_child() {
    let original = "* aaa\n* bbb\n  * ccc\n  * ddd\n";
    let (mut app, _f, _, _) = app_with_page(original);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bbb");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    let tab = key_event(Key::Named(Named::Tab), Modifiers::empty());
    send(&mut app, Message::Window(id, WindowEvent::Key(tab)));
    assert_eq!(page_text(&app, id), "* aaa\n  * bbb\n    * ccc\n  * ddd\n");
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 2)));

    command(&mut app, id, EditorCommand::Undo);
    assert_eq!(page_text(&app, id), original);
}

#[test]
fn tab_moves_selected_siblings_as_a_group() {
    let original = "> - aaa\n> - bbb\n> - ccc\n> - ddd\n";
    let (mut app, _f, _, _) = app_with_page(original);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "ccc");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftUp));
    let page = |app: &App| app.editors[&id].page.selected_blocks();
    assert_eq!(page(&app), Some((2, 1)));
    let tab = key_event(Key::Named(Named::Tab), Modifiers::empty());
    send(&mut app, Message::Window(id, WindowEvent::Key(tab)));
    assert_eq!(page_text(&app, id), "> - aaa\n>   - bbb\n>   - ccc\n> - ddd\n");
    assert_eq!(page(&app), Some((2, 1)));

    let tab = key_event(Key::Named(Named::Tab), Modifiers::SHIFT);
    send(&mut app, Message::Window(id, WindowEvent::Key(tab)));
    assert_eq!(page_text(&app, id), original);
    assert_eq!(page(&app), Some((2, 1)));

    command(&mut app, id, EditorCommand::Undo);
    assert_eq!(page_text(&app, id), "> - aaa\n>   - bbb\n>   - ccc\n> - ddd\n");
    command(&mut app, id, EditorCommand::Undo);
    assert_eq!(page_text(&app, id), original);
}

#[test]
fn tab_aligns_selected_siblings_with_different_marker_columns() {
    let (mut app, _f, _, _) = app_with_page("   * aaa\n   * bbb\n* ccc\n* ddd\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bbb");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    let tab = key_event(Key::Named(Named::Tab), Modifiers::empty());
    send(&mut app, Message::Window(id, WindowEvent::Key(tab)));
    assert_eq!(page_text(&app, id), "   * aaa\n     * bbb\n     * ccc\n* ddd\n");
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 2)));
}

#[test]
fn tab_keeps_unselected_ordered_items_at_their_depth() {
    let original = "100. aaa\n1. bbb\n   1. ccc\n      1. eee\n   2. ddd\n      1. fff\n";
    let (mut app, _f, _, _) = app_with_page(original);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bbb");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    let tab = key_event(Key::Named(Named::Tab), Modifiers::empty());
    send(&mut app, Message::Window(id, WindowEvent::Key(tab)));

    let page = &app.editors[&id].page;
    assert_eq!(page.text(), "100. aaa\n     1. bbb\n        1. ccc\n        1. eee\n     2. ddd\n        1. fff\n");
    assert_eq!(page.selected_blocks(), Some((1, 2)));
    let depths: Vec<_> = page
        .doc()
        .blocks
        .iter()
        .map(|block| match &block.kind {
            crate::markdown::BlockKind::ListItem(item) => item.depth,
            _ => panic!("expected a list item"),
        })
        .collect();
    assert_eq!(depths, [0, 1, 2, 2, 1, 2]);

    command(&mut app, id, EditorCommand::Undo);
    assert_eq!(page_text(&app, id), original);
}

#[test]
fn tab_preserves_task_checkboxes_with_and_without_a_selection() {
    let original = "- [ ] aaa\n- [x] bbb\n  - [x] ccc\n  - [ ] ddd\n";
    let (mut app, _f, _, _) = app_with_page(original);
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bbb");
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(page_text(&app, id), "- [ ] aaa\n- [x] bbb\n  - [ ] ddd\n    - [x] ccc\n");

    command(&mut app, id, EditorCommand::Undo);
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    let tab = key_event(Key::Named(Named::Tab), Modifiers::empty());
    send(&mut app, Message::Window(id, WindowEvent::Key(tab)));
    assert_eq!(page_text(&app, id), "- [ ] aaa\n  - [x] bbb\n    - [x] ccc\n  - [ ] ddd\n");
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 2)));
}

#[test]
fn tab_keeps_an_ordered_child_as_a_list_item() {
    let (mut app, _f, _, _) = app_with_page("* aaa\n* bbb\n  1. ccc\n  2. ddd\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bbb");
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(page_text(&app, id), "* aaa\n* bbb\n  1. ddd\n\n     2. ccc\n");
    let blocks = &app.editors[&id].page.doc().blocks;
    assert!(matches!(&blocks[3].kind, crate::markdown::BlockKind::ListItem(item) if item.depth == 2));
}

#[test]
fn tab_respects_selection_and_list_boundaries() {
    let original = "- aaa\n- bbb\n  - child\n- ccc\n";
    let (mut app, _f, _, _) = app_with_page(original);
    let id = open_with_f1(&mut app);
    select(&mut app, id, "bbb");
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(page_text(&app, id), "- aaa\n  - bbb\n    - child\n- ccc\n");

    click_after(&mut app, id, "child");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
    assert_eq!(page_text(&app, id), "- aaa\n  - bbb\n    - child\n- ccc\n");
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((2, 3)));
}

/// Ctrl+Up / Ctrl+Down, as the window gets them.
fn ctrl_arrow(app: &mut App, id: window::Id, down: bool) {
    let key = Key::Named(if down { Named::ArrowDown } else { Named::ArrowUp });
    send(app, Message::Window(id, WindowEvent::Key(key_event(key, Modifiers::CTRL))));
}

#[test]
fn ctrl_up_and_down_move_a_task_with_the_cursor() {
    let (mut app, _f, _, _) = app_with_page("- [x] a\n- [ ] b\n  - b1\n- [ ] c\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "] b");
    ctrl_arrow(&mut app, id, false);
    assert_eq!(page_text(&app, id), "- [ ] b\n  - b1\n- [x] a\n- [ ] c\n");
    // The cursor moved with it: typing goes on in the item.
    typing(&mut app, id, "!");
    assert_eq!(page_text(&app, id), "- [ ] b!\n  - b1\n- [x] a\n- [ ] c\n");
    // At the top of the list, Ctrl+Up changes nothing.
    ctrl_arrow(&mut app, id, false);
    assert_eq!(page_text(&app, id), "- [ ] b!\n  - b1\n- [x] a\n- [ ] c\n");
    ctrl_arrow(&mut app, id, true);
    ctrl_arrow(&mut app, id, true);
    assert_eq!(page_text(&app, id), "- [x] a\n- [ ] c\n- [ ] b!\n  - b1\n");
    command(&mut app, id, EditorCommand::Undo);
    assert_eq!(page_text(&app, id), "- [x] a\n- [ ] b!\n  - b1\n- [ ] c\n");
}

#[test]
fn ctrl_up_outside_a_list_and_in_an_ordered_list() {
    let (mut app, _f, _, _) = app_with_page("one\n\n1. a\n2. b\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    ctrl_arrow(&mut app, id, true);
    assert_eq!(page_text(&app, id), "one\n\n1. a\n2. b\n");
    click_after(&mut app, id, "2. b");
    ctrl_arrow(&mut app, id, false);
    assert_eq!(page_text(&app, id), "one\n\n1. b\n2. a\n");
}

#[test]
fn moving_an_item_in_the_source_mode() {
    let (mut app, _f, _, _) = app_with_page("- a\n- b\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "- b");
    command(&mut app, id, EditorCommand::SourceMode);
    ctrl_arrow(&mut app, id, false);
    assert_eq!(page_text(&app, id), "- b\n- a\n");
    assert_eq!(app.editors[&id].source.as_ref().map(|s| s.text()).as_deref(), Some("- b\n- a\n"));
}
