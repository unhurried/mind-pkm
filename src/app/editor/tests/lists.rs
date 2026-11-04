//! The blank lines of the lists the editor edits: one form for all the
//! items of a list, loose only where a block needs a blank line.

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
