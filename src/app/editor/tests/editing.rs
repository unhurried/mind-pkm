//! Editing the page: typing, clicks and arrows across blocks, shortcuts, whole blocks, source mode, code blocks.

use super::*;

/// A key typed with `text` in the editor window.
fn type_key(app: &mut App, id: window::Id, c: char) {
    let text = c.to_string();
    send(app, Message::KeyIn(id, typing_event(Key::Character(text.as_str().into()), Modifiers::empty(), &text)));
}

#[test]
fn typing_with_nothing_active_types_into_the_first_block() {
    let (mut app, _f, _, _) = app_with_page("# Page1\n\nHello world.\n");
    let id = open_with_f1(&mut app);
    assert!(app.editors[&id].page.active().is_none());
    // The first key starts editing and is typed too; then the widget has the keys.
    type_key(&mut app, id, 'a');
    typing(&mut app, id, "bc");
    assert_eq!(page_text(&app, id), "# abcPage1\n\nHello world.\n");
}

#[test]
fn end_with_nothing_active_goes_to_the_end_of_the_first_block() {
    let (mut app, _f, _, _) = app_with_page("# Page1\n\nHello world.\n");
    let id = open_with_f1(&mut app);
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::End), Modifiers::empty())));
    // The block is active: the widget has the keys.
    typing(&mut app, id, " unsaved");
    assert_eq!(page_text(&app, id), "# Page1 unsaved\n\nHello world.\n");
}

#[test]
fn typing_enter_and_undo() {
    let (mut app, _f, _, _) = app_with_page("- one\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "two");
    assert_eq!(page_text(&app, id), "- one\n- two\n");
    assert_eq!(app.title(id), "Plan.md * - Mind PKM");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "- one\n- \n");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "- one\n");
}

#[test]
fn an_empty_template_item_is_typed_into() {
    let (mut app, _f, _, _) = app_with_page("### Next Steps\r\n\r\n* \r\n");
    let id = open_with_f1(&mut app);
    // A click beside the item's (missing) text, on its bullet.
    let item = app.editors[&id].page.doc().blocks.len() - 1;
    editor(&mut app, id, EditorMsg::BlockPressed(item));
    typing(&mut app, id, "step");
    assert_eq!(page_text(&app, id), "### Next Steps\n\n* step\n");
}

#[test]
fn a_click_in_the_gap_between_blocks_goes_to_the_nearer_block() {
    let (mut app, _f, _, _) = app_with_page("* one\n* two\n\nAfter\n");
    let id = open_with_f1(&mut app);
    let blocks = app.editors[&id].page.doc().blocks.clone();
    // The upper half of the gap: the end of the block above.
    editor(&mut app, id, EditorMsg::PressedAt(blocks[1].range.end));
    typing(&mut app, id, "!");
    assert_eq!(page_text(&app, id), "* one\n* two!\n\nAfter\n");
    // The lower half: the start of the block below.
    let below = app.editors[&id].page.doc().blocks[2].range.start;
    editor(&mut app, id, EditorMsg::PressedAt(below));
    typing(&mut app, id, "?");
    assert_eq!(page_text(&app, id), "* one\n* two!\n\n?After\n");
}

#[test]
fn a_click_right_of_a_line_with_math_puts_the_cursor_at_its_end() {
    let (mut app, _f, _, _) = app_with_page("Inline $x^2+\\frac{1}{2}$ math\nnext $y$\n");
    let id = open_with_f1(&mut app);
    // The view reports the end of the clicked line: its last inline's end.
    let end_of = |app: &App, line: usize| {
        let editor = &app.editors[&id];
        let doc = editor.page.doc();
        let inlines = crate::markdown::content::inlines(doc, doc.block_events(0));
        inlines
            .split(|i| matches!(i, crate::markdown::content::Inline::Break { .. }))
            .nth(line)
            .unwrap()
            .last()
            .unwrap()
            .source()
            .end
    };
    let first = end_of(&app, 0);
    editor(&mut app, id, EditorMsg::PressedAt(first));
    typing(&mut app, id, "!");
    assert_eq!(page_text(&app, id), "Inline $x^2+\\frac{1}{2}$ math!\nnext $y$\n");
    let second = end_of(&app, 1);
    editor(&mut app, id, EditorMsg::PressedAt(second));
    typing(&mut app, id, "?");
    assert_eq!(page_text(&app, id), "Inline $x^2+\\frac{1}{2}$ math!\nnext $y$?\n");
}

#[test]
fn arrows_cross_into_the_next_and_previous_blocks() {
    let (mut app, _f, _, _) = app_with_page("first line\n\nsecond\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "first");
    editor(&mut app, id, EditorMsg::Bind(Bind::Down));
    let page = &app.editors[&id].page;
    assert_eq!(page.region_text(), Some("second"));
    editor(&mut app, id, EditorMsg::Bind(Bind::Up));
    assert_eq!(app.editors[&id].page.region_text(), Some("first line"));
    // Right at the end and Left at the start cross too.
    click_after(&mut app, id, "first line");
    editor(&mut app, id, EditorMsg::Bind(Bind::Right));
    assert_eq!(app.editors[&id].page.region_text(), Some("second"));
    editor(&mut app, id, EditorMsg::Bind(Bind::Left));
    assert_eq!(app.editors[&id].page.region_text(), Some("first line"));
}

#[test]
fn backspace_at_the_start_joins_paragraphs() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n");
    let id = open_with_f1(&mut app);
    editor(&mut app, id, EditorMsg::PressedAt(5));
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "onetwo\n");
    assert_eq!(app.editors[&id].page.sel().head, 3);
}

#[test]
fn a_task_box_is_checked_by_a_click() {
    let (mut app, _f, _, _) = app_with_page("- [ ] call\n");
    let id = open_with_f1(&mut app);
    editor(&mut app, id, EditorMsg::TaskToggled(0));
    assert_eq!(page_text(&app, id), "- [x] call\n");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    assert_eq!(page_text(&app, id), "- [ ] call\n");
}

#[test]
fn paragraph_shortcuts() {
    let (mut app, _f, _, _) = app_with_page("Plan\n\none\n\ntwo\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "Pl");
    press(&mut app, id, "2", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "## Plan\n\none\n\ntwo\n");
    press(&mut app, id, "=", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "# Plan\n\none\n\ntwo\n");
    press(&mut app, id, "0", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "Plan\n\none\n\ntwo\n");
    // Two blocks selected become a bullet list.
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 2)));
    press(&mut app, id, "]", Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(page_text(&app, id), "Plan\n\n- one\n- two\n");
    click_after(&mut app, id, "two");
    press(&mut app, id, "q", Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(page_text(&app, id), "Plan\n\n- one\n> - two\n");
}

#[test]
fn format_shortcuts_and_clearing() {
    let (mut app, _f, _, _) = app_with_page("a word here\n");
    let id = open_with_f1(&mut app);
    select(&mut app, id, "word");
    press(&mut app, id, "b", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a **word** here\n");
    press(&mut app, id, "b", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a word here\n");
    // The word at the cursor.
    click_after(&mut app, id, "he");
    press(&mut app, id, "i", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a word *here*\n");
    select(&mut app, id, "word");
    press(&mut app, id, "k", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a [word]() *here*\n");
    // Ctrl+\ clears the formatting of the block.
    click_after(&mut app, id, "a ");
    press(&mut app, id, "\\", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "a [word]() here\n");
}

#[test]
fn auto_pairing_can_be_turned_off() {
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    editor(&mut app, id, EditorMsg::Bind(Bind::Type(' ')));
    editor(&mut app, id, EditorMsg::Bind(Bind::Type('(')));
    assert_eq!(page_text(&app, id), "x ()\n");
    editor(&mut app, id, EditorMsg::Bind(Bind::Type(')')));
    assert_eq!(page_text(&app, id), "x ()\n");
    editor(&mut app, id, EditorMsg::Command(EditorCommand::ToggleAutoPair));
    assert!(!app.editor_prefs.auto_pair);
    editor(&mut app, id, EditorMsg::Bind(Bind::Type('[')));
    assert_eq!(page_text(&app, id), "x ()[\n");
}

#[test]
fn selection_helpers() {
    let (mut app, _f, _, _) = app_with_page("some **bold text** and more\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "bo");
    press(&mut app, id, "e", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "bold text");
    click_after(&mut app, id, "mo");
    press(&mut app, id, "d", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "more");
    press(&mut app, id, "l", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "some **bold text** and more");
}

#[test]
fn ctrl_click_on_an_anchor_goes_to_the_heading() {
    let (mut app, _f, _, _) = app_with_page("[to b](#part-b)\n\n# Part A\n\n# Part B\n");
    let id = open_with_f1(&mut app);
    send(&mut app, Message::KeyIn(id, keyboard::Event::ModifiersChanged(Modifiers::CTRL)));
    editor(&mut app, id, EditorMsg::Link("#part-b".into(), 1));
    let page = &app.editors[&id].page;
    assert!(page.active().is_none());
    assert_eq!(page.sel().head, page.text().find("# Part B").unwrap());
}

#[test]
fn dragging_onto_other_blocks_selects_them() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n\nfour\n");
    let id = open_with_f1(&mut app);
    // A press in a block's text, then the pointer onto the blocks below and back.
    click_after(&mut app, id, "two");
    live(&mut app, id, LiveMsg::Click { at: 1, extend: false });
    assert_eq!(app.editors[&id].page.selected_blocks(), None);
    editor(&mut app, id, EditorMsg::BlockEntered(3));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 3)));
    assert!(app.editors[&id].page.active().is_none());
    editor(&mut app, id, EditorMsg::BlockEntered(2));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 2)));
    editor(&mut app, id, EditorMsg::BlockEntered(1));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 1)));
    editor(&mut app, id, EditorMsg::BlockEntered(0));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 0)));
    // Released: moving over blocks changes nothing.
    editor(&mut app, id, EditorMsg::Released);
    editor(&mut app, id, EditorMsg::BlockEntered(3));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((1, 0)));
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::Delete), Modifiers::empty())));
    assert_eq!(page_text(&app, id), "three\n\nfour\n");

    // A press on a rendered block, beside its text.
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n");
    let id = open_with_f1(&mut app);
    editor(&mut app, id, EditorMsg::BlockPressed(2));
    editor(&mut app, id, EditorMsg::BlockEntered(0));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((2, 0)));
    editor(&mut app, id, EditorMsg::Released);
    // In source mode, no block is selected.
    editor(&mut app, id, EditorMsg::Command(EditorCommand::SourceMode));
    editor(&mut app, id, EditorMsg::Action(Action::Click(iced::Point::ORIGIN)));
    editor(&mut app, id, EditorMsg::BlockEntered(1));
    assert_eq!(app.editors[&id].page.selected_blocks(), None);
}

#[test]
fn whole_blocks_are_selected_and_replaced() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "two");
    // Ctrl+A selects the block's text, a second time all blocks.
    press(&mut app, id, "a", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "two");
    press(&mut app, id, "a", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((0, 2)));
    // Shift+click from the active block.
    click_after(&mut app, id, "one");
    send(&mut app, Message::KeyIn(id, keyboard::Event::ModifiersChanged(Modifiers::SHIFT)));
    editor(&mut app, id, EditorMsg::BlockPressed(1));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((0, 1)));
    send(&mut app, Message::KeyIn(id, keyboard::Event::ModifiersChanged(Modifiers::empty())));
    // Typing replaces them.
    send(&mut app, Message::KeyIn(id, typing_event(Key::Character("z".into()), Modifiers::empty(), "z")));
    assert_eq!(page_text(&app, id), "z\n\nthree\n");
}

#[test]
fn source_mode_keeps_the_cursor_and_the_history() {
    let (mut app, _f, _, _) = app_with_page("# one\n\ntwo\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "tw");
    press(&mut app, id, "/", Modifiers::CTRL);
    let source = app.editors[&id].source.as_ref().expect("source mode");
    assert_eq!(source.text(), "# one\n\ntwo\n");
    assert_eq!(crate::app::editor::page::content_sel(source, &source.text()).head, 9);
    typing(&mut app, id, "x");
    assert_eq!(page_text(&app, id), "# one\n\ntwxo\n");
    assert_eq!(app.editors[&id].counts().0.words, 2, "the status bar counts the page");
    // Back: the cursor where it was in the source; the typing can be undone.
    press(&mut app, id, "/", Modifiers::CTRL);
    assert!(app.editors[&id].source.is_none());
    assert_eq!(app.editors[&id].page.sel().head, 10);
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "# one\n\ntwo\n");
}

#[test]
fn counting_a_selection() {
    let (mut app, _f, _, _) = app_with_page("**two words** and 日本語\n");
    let id = open_with_f1(&mut app);
    let (all, none) = app.editors[&id].counts();
    assert_eq!((all.words, all.characters, none), (6, 17, None));
    select(&mut app, id, "two words");
    let (_, selected) = app.editors[&id].counts();
    let selected = selected.unwrap();
    assert_eq!((selected.words, selected.characters), (2, 9));
}

#[test]
fn deleting_three_blocks() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n\nfour\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    // From the text editor's last line to the next block; then the window's keys.
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::ArrowDown), Modifiers::SHIFT)));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((0, 2)));
    send(&mut app, Message::KeyIn(id, key_event(Key::Named(Named::Delete), Modifiers::empty())));
    assert_eq!(page_text(&app, id), "four\n");
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(page_text(&app, id), "one\n\ntwo\n\nthree\n\nfour\n");
}

/// The text the text editor of the block being edited holds.
fn editor_text(app: &App, id: window::Id) -> String {
    app.editors[&id].page.active().and_then(|a| a.content()).map(|c| c.text()).unwrap_or_default()
}

#[test]
fn code_blocks_are_edited_without_their_fences() {
    let (mut app, _f, _, _) = app_with_page("> ```rust\n> let x = 1;\n> ```\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "let x = 1;");
    assert_eq!(editor_text(&app, id), "let x = 1;");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "x");
    assert_eq!(page_text(&app, id), "> ```rust\n> let x = 1;\n> x\n> ```\n");
    assert_eq!(editor_text(&app, id), "let x = 1;\nx");
    // Its language.
    editor(&mut app, id, EditorMsg::CodeLanguage("python".into()));
    assert_eq!(page_text(&app, id), "> ```python\n> let x = 1;\n> x\n> ```\n");
    assert_eq!(app.editors[&id].page.sel().head, page_text(&app, id).find("\n> ```\n").unwrap());
    // Undo takes back the language, the typing and the line break.
    for _ in 0..3 {
        editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
    }
    assert_eq!(page_text(&app, id), "> ```rust\n> let x = 1;\n> ```\n");
    assert_eq!(editor_text(&app, id), "let x = 1;");
}

#[test]
fn leaving_and_removing_code_blocks() {
    use crate::markdown::BlockKind;
    // Backspace at the start of the code: to the end of the block before.
    let (mut app, _f, _, _) = app_with_page("a\n\n```\ncode\n```\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "```\n");
    assert_eq!(editor_text(&app, id), "code");
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert_eq!(page_text(&app, id), "a\n\n```\ncode\n```\n");
    assert_eq!(app.editors[&id].page.sel().head, 1);
    // Down at the last line of the code ending the page: a new paragraph after it.
    click_after(&mut app, id, "cod");
    editor(&mut app, id, EditorMsg::Bind(Bind::Down));
    assert!(app.editors[&id].page.pending().is_some());
    typing(&mut app, id, "z");
    assert_eq!(page_text(&app, id), "a\n\n```\ncode\n```\n\nz\n");
    // Backspace in an empty code block: an empty paragraph in its place.
    let (mut app, _f, _, _) = app_with_page("a\n\n```\n```\n\nb\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "a\n\n```");
    assert!(matches!(active_kind(&app, id), BlockKind::Code { .. }));
    assert_eq!(editor_text(&app, id), "");
    editor(&mut app, id, EditorMsg::Bind(Bind::Backspace));
    assert!(app.editors[&id].page.pending().is_some());
    typing(&mut app, id, "p");
    assert_eq!(page_text(&app, id), "a\n\np\n\nb\n");
}

#[test]
fn code_typed_into_an_empty_code_block() {
    let (mut app, _f, _, _) = app_with_page("```js\n```\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "```js");
    typing(&mut app, id, "x;");
    assert_eq!(page_text(&app, id), "```js\nx;\n```\n");
    assert_eq!(editor_text(&app, id), "x;");
}
