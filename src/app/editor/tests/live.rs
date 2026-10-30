//! Blocks edited in their rendered form: what they show, and how typed markers become elements.

use super::*;

fn shown(app: &App, id: window::Id) -> String {
    app.editors[&id]
        .page
        .active
        .as_ref()
        .and_then(|a| a.live.as_ref())
        .map(|l| l.display.text.clone())
        .unwrap_or_default()
}

#[test]
fn only_the_element_at_the_cursor_shows_its_markdown() {
    let (mut app, _f, _, _) = app_with_page("some **bold** [link](x.md)\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "so");
    assert_eq!(shown(&app, id), "some bold link");
    click_after(&mut app, id, "**bo");
    assert_eq!(shown(&app, id), "some **bold** link");
    // Clicking the rendered text puts the cursor there.
    live(&mut app, id, LiveMsg::Click { at: 2, extend: false });
    assert_eq!(app.editors[&id].page.sel.head, 2);
    assert_eq!(shown(&app, id), "some bold link");
}

#[test]
fn headings_and_list_items_are_edited_without_their_markers() {
    let (mut app, _f, _, _) = app_with_page("## Plan\n\n- [ ] task\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "Pl");
    assert_eq!(shown(&app, id), "Plan");
    // Backspace at the start of the heading's text: a paragraph.
    live(&mut app, id, LiveMsg::MoveTo { at: 0, select: false });
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "Plan\n\n- [ ] task\n");
    click_after(&mut app, id, "ta");
    assert_eq!(shown(&app, id), "task");
    typing(&mut app, id, "X");
    assert_eq!(page_text(&app, id), "Plan\n\n- [ ] taXsk\n");
}

#[test]
fn deleting_rendered_text_keeps_or_removes_its_markup() {
    let (mut app, _f, _, _) = app_with_page("**bold** x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    assert_eq!(shown(&app, id), "bold x");
    // Selecting `bo` of the rendered word and deleting it.
    live(&mut app, id, LiveMsg::Click { at: 0, extend: false });
    let l = app.editors[&id].page.active.as_ref().unwrap().live.clone().unwrap();
    let start = l.display.text.find("bo").unwrap();
    live(&mut app, id, LiveMsg::Click { at: start, extend: false });
    live(&mut app, id, LiveMsg::Drag(start + 2));
    editor(&mut app, id, EditorMsg::Bind(Bind::Delete));
    assert_eq!(page_text(&app, id), "**ld** x\n");

    let (mut app, _f, _, _) = app_with_page("a **bold** b\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "b\n".trim_end());
    click_after(&mut app, id, "a");
    assert_eq!(shown(&app, id), "a bold b");
    live(&mut app, id, LiveMsg::Click { at: 2, extend: false });
    // From the bold's start (now shown with its markers) to its end.
    let l = app.editors[&id].page.active.as_ref().unwrap().live.clone().unwrap();
    let (s, e) = (l.display.text.find("**").unwrap(), l.display.text.rfind("**").unwrap() + 2);
    live(&mut app, id, LiveMsg::Click { at: s, extend: false });
    live(&mut app, id, LiveMsg::Drag(e));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "a  b\n");
}

#[test]
fn enter_twice_leaves_a_quote() {
    let (mut app, _f, _, _) = app_with_page("> one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    // A new paragraph in the quote, without source until typed into.
    let pending = app.editors[&id].page.pending.clone().expect("a new paragraph");
    assert_eq!(pending.outer, vec![crate::markdown::parse::ContainerKind::Quote]);
    assert_eq!(page_text(&app, id), "> one\n");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "after");
    assert_eq!(page_text(&app, id), "> one\n\nafter\n");
    // Typed into right away, it stays in the quote.
    let (mut app, _f, _, _) = app_with_page("> one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "two");
    assert_eq!(page_text(&app, id), "> one\n>\n> two\n");
    assert_eq!(shown(&app, id), "two");
}

#[test]
fn moving_by_characters_and_words_in_rendered_text() {
    use iced::widget::text_editor::Motion;
    let (mut app, _f, _, _) = app_with_page("ab **cd** ef\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "a");
    let right = |app: &mut App| live(app, id, LiveMsg::Motion { motion: Motion::Right, select: false });
    right(&mut app);
    right(&mut app);
    // Next to the bold: its markers show, the cursor before them.
    assert_eq!(shown(&app, id), "ab **cd** ef");
    assert_eq!(app.editors[&id].page.sel.head, 3);
    live(&mut app, id, LiveMsg::Motion { motion: Motion::WordRight, select: false });
    live(&mut app, id, LiveMsg::Motion { motion: Motion::End, select: true });
    assert!(app.editors[&id].page.selected_source().ends_with("ef"));
}

#[test]
fn typing_into_a_new_paragraph() {
    let (mut app, _f, _, _) = app_with_page("last\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "last");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    for c in ["q", "2"] {
        live(&mut app, id, LiveMsg::Insert(c.into()));
    }
    assert_eq!(page_text(&app, id), "last\n\nq2\n");
}

/// Types `s` as the editing widget sends it: characters auto-pairing may
/// apply to as key bindings, the others as inserted text.
fn type_keys(app: &mut App, id: window::Id, s: &str) {
    for c in s.chars() {
        if "()[]{}\"'`*_~$".contains(c) {
            editor(app, id, EditorMsg::Bind(Bind::Type(c)));
        } else {
            live(app, id, LiveMsg::Insert(c.into()));
        }
    }
}

/// An editor on `text` with the cursor in a new paragraph after its first line.
fn new_paragraph_after_first(text: &str) -> (App, window::Id, crate::pkm::testutil::Fixture) {
    let (mut app, f, _, _) = app_with_page(text);
    let id = open_with_f1(&mut app);
    let first = text.lines().next().unwrap().to_string();
    click_after(&mut app, id, &first);
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    (app, id, f)
}

#[test]
fn a_marker_shows_as_typed_until_its_space() {
    use crate::markdown::BlockKind;
    let (mut app, id, _f) = new_paragraph_after_first("a\n");
    type_keys(&mut app, id, "#");
    assert_eq!((shown(&app, id), active_kind(&app, id)), ("#".into(), BlockKind::Paragraph));
    type_keys(&mut app, id, "tag");
    assert_eq!((page_text(&app, id), shown(&app, id)), ("a\n\n#tag\n".into(), "#tag".into()));

    let (mut app, id, _f) = new_paragraph_after_first("a\n");
    type_keys(&mut app, id, "*");
    assert_eq!((shown(&app, id), active_kind(&app, id)), ("*".into(), BlockKind::Paragraph));
    type_keys(&mut app, id, " ");
    assert!(matches!(active_kind(&app, id), BlockKind::ListItem(_)));
    assert_eq!(shown(&app, id), "");
    type_keys(&mut app, id, "item");
    assert_eq!((page_text(&app, id), shown(&app, id)), ("a\n\n* item\n".into(), "item".into()));

    // `##`, `1.`, `>` and a task box wait for their space too.
    // `- ` is a list item already: then its box waits for its space.
    for (keys, before, kind) in
        [("## ", "##", "Heading"), ("1. ", "1.", "ListItem"), ("> ", ">", "Quote"), ("- [ ] ", "[ ]", "ListItem")]
    {
        let (mut app, id, _f) = new_paragraph_after_first("a\n");
        type_keys(&mut app, id, keys.trim_end());
        assert_eq!(shown(&app, id), before, "{keys}");
        type_keys(&mut app, id, " ");
        assert_eq!(shown(&app, id), "", "{keys}");
        assert!(format!("{:?}", active_kind(&app, id)).starts_with(kind), "{keys}: {:?}", active_kind(&app, id));
    }
    let (mut app, id, _f) = new_paragraph_after_first("a\n");
    type_keys(&mut app, id, "- [ ] x");
    let item = app.editors[&id].page.doc.blocks.iter().find_map(|b| match &b.kind {
        BlockKind::ListItem(i) => Some(i.task),
        _ => None,
    });
    assert_eq!(item, Some(Some(false)));
}

#[test]
fn a_list_in_a_quote_as_it_is_typed() {
    use crate::markdown::BlockKind;
    use crate::markdown::parse::ContainerKind;
    let (mut app, id, _f) = new_paragraph_after_first("a\n");
    type_keys(&mut app, id, "> - x");
    assert_eq!(page_text(&app, id), "a\n\n> - x\n");
    assert_eq!(shown(&app, id), "x");
    let page = &app.editors[&id].page;
    let b = page.active_block().unwrap();
    assert!(matches!(page.doc.blocks[b].kind, BlockKind::ListItem(_)));
    assert_eq!(page.doc.outer_kinds(b), vec![ContainerKind::Quote]);
    // Enter: the next item in the quote.
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    type_keys(&mut app, id, "y");
    assert_eq!(page_text(&app, id), "a\n\n> - x\n> - y\n");
}

#[test]
fn line_syntax_waits_for_enter() {
    use crate::markdown::BlockKind;
    // A fence leaves the paragraphs after it alone until Enter closes it.
    let (mut app, id, _f) = new_paragraph_after_first("a\n\nb\n\nc\n");
    type_keys(&mut app, id, "```");
    assert_eq!(page_text(&app, id), "a\n\n```\n\nb\n\nc\n");
    assert_eq!(shown(&app, id), "```");
    let kinds: Vec<BlockKind> = app.editors[&id].page.doc.blocks.iter().map(|b| b.kind.clone()).collect();
    assert_eq!(kinds, vec![BlockKind::Paragraph; 4]);
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    assert_eq!(page_text(&app, id), "a\n\n```\n\n```\n\nb\n\nc\n");
    assert!(matches!(active_kind(&app, id), BlockKind::Code { .. }));

    // A thematic break, then a new paragraph after it.
    let (mut app, id, _f) = new_paragraph_after_first("a\n");
    type_keys(&mut app, id, "---");
    assert_eq!(shown(&app, id), "---");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    let page = &app.editors[&id].page;
    assert!(page.doc.blocks.iter().any(|b| b.kind == BlockKind::ThematicBreak));
    assert!(page.pending.is_some());

    // A setext underline.
    let (mut app, _f, _, _) = app_with_page("Title\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "Title");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftEnter));
    type_keys(&mut app, id, "==");
    assert_eq!(shown(&app, id), "Title\n==");
}

#[test]
fn an_undetermined_marker_left_behind_is_what_the_text_says() {
    use crate::markdown::BlockKind;
    let (mut app, id, _f) = new_paragraph_after_first("a\n");
    type_keys(&mut app, id, "#");
    click_after(&mut app, id, "a");
    let kinds: Vec<BlockKind> = app.editors[&id].page.doc.blocks.iter().map(|b| b.kind.clone()).collect();
    assert_eq!(kinds, vec![BlockKind::Paragraph, BlockKind::Heading(1)]);
    // Undo takes the marker back as typed.
    assert!(app.editors.get_mut(&id).unwrap().page.undo());
    assert_eq!(page_text(&app, id), "a\n");
}

#[test]
fn shift_enter_at_the_end_of_a_paragraph() {
    let (mut app, _f, _, _) = app_with_page("one\n\nnext\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftEnter));
    // The cursor on a new line of the paragraph.
    assert!(app.editors[&id].page.pending.is_none());
    assert_eq!(shown(&app, id), "one\n");
    type_keys(&mut app, id, "two");
    assert_eq!(page_text(&app, id), "one\ntwo\n\nnext\n");
    assert_eq!(shown(&app, id), "one\ntwo");
    // In a quote too.
    let (mut app, _f, _, _) = app_with_page("> one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftEnter));
    type_keys(&mut app, id, "two");
    assert_eq!(page_text(&app, id), "> one\n> two\n");
}

#[test]
fn paragraph_shortcuts_inside_a_quote() {
    use EditorCommand as C;
    let (mut app, _f, _, _) = app_with_page("> Plan\n>\n> item\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "Pl");
    editor(&mut app, id, EditorMsg::Command(C::Heading(2)));
    assert_eq!(page_text(&app, id), "> ## Plan\n>\n> item\n");
    assert_eq!(shown(&app, id), "Plan");
    click_after(&mut app, id, "ite");
    editor(&mut app, id, EditorMsg::Command(C::BulletList));
    assert_eq!(page_text(&app, id), "> ## Plan\n>\n> - item\n");
    // Backspace at the start of the item removes its bullet; it stays in the quote.
    live(&mut app, id, LiveMsg::MoveTo { at: 0, select: false });
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "> ## Plan\n>\n> item\n");
}

#[test]
fn an_item_indented_by_tab_is_an_empty_item_until_typed_into() {
    use crate::markdown::BlockKind;
    for marker in ["*", "-", "+"] {
        let (mut app, id, _f) = new_paragraph_after_first("a\n");
        type_keys(&mut app, id, &format!("{marker} aaa"));
        for (depth, text) in [(1, "bbb"), (2, "ccc")] {
            editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
            editor(&mut app, id, EditorMsg::Bind(Bind::Tab));
            // The empty item is an item, not a line of the paragraph above it.
            assert_eq!(shown(&app, id), "", "{marker} {text}");
            assert!(matches!(active_kind(&app, id), BlockKind::ListItem(ref i) if i.depth == depth), "{marker} {text}");
            type_keys(&mut app, id, text);
            assert_eq!(shown(&app, id), text, "{marker} {text}");
        }
        assert_eq!(page_text(&app, id), format!("a\n\n{marker} aaa\n  {marker} bbb\n    {marker} ccc\n"));
    }
}

#[test]
fn blocks_typed_under_a_list_item() {
    // Enter, then Backspace in the new empty item: a paragraph of the item above.
    let (mut app, _f, _, _) = app_with_page("* aaa\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "aaa");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    assert_eq!(page_text(&app, id), "* aaa\n* \n");
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "* aaa\n");
    let pending = app.editors[&id].page.pending.clone().expect("a new paragraph");
    assert_eq!(pending.outer, vec![crate::markdown::parse::ContainerKind::Item { depth: 0, indent: 2 }]);
    type_keys(&mut app, id, "| bbb | ccc |");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    assert_eq!(page_text(&app, id), "* aaa\n\n  | bbb | ccc |\n  | --- | --- |\n  |  |  |\n");
    assert_eq!(active_kind(&app, id), crate::markdown::BlockKind::Table);
    let doc = crate::markdown::parse::Document::parse(&page_text(&app, id));
    assert_eq!(doc.blocks[1].kind, crate::markdown::BlockKind::Table);
    assert_eq!(doc.blocks[1].outer, vec![0]);
    // Typing into a cell keeps the table in the item.
    type_keys(&mut app, id, "x");
    assert_eq!(page_text(&app, id), "* aaa\n\n  | bbb | ccc |\n  | --- | --- |\n  | x   |     |\n");
    // A code block the same way, between the item and the next one.
    let (mut app, _f, _, _) = app_with_page("- a\n- b\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "a");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    type_keys(&mut app, id, "```");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "x");
    assert_eq!(page_text(&app, id), "- a\n\n  ```\n  x\n  ```\n\n- b\n");
    // A table in a quote keeps its `>` the same way.
    let (mut app, _f, _, _) = app_with_page("> | a | b |\n> | --- | --- |\n> |  |  |\n");
    let id = open_with_f1(&mut app);
    let cell = page_text(&app, id).rfind("> |").unwrap() + 3;
    editor(&mut app, id, EditorMsg::PressedAt(cell));
    type_keys(&mut app, id, "y");
    assert_eq!(page_text(&app, id), "> | a   | b   |\n> | --- | --- |\n> | y   |     |\n");
    // Backspace again takes the new paragraph back, to the end of the item.
    let (mut app, _f, _, _) = app_with_page("* aaa\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "aaa");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    typing(&mut app, id, "!");
    assert_eq!(page_text(&app, id), "* aaa!\n");
}
