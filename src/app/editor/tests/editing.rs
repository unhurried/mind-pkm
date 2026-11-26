//! Editing the page: typing, clicks and arrows across blocks, shortcuts, whole blocks, source mode, code blocks.

use super::*;

/// A key typed with `text` in the editor window.
fn type_key(app: &mut App, id: window::Id, c: char) {
    let text = c.to_string();
    send(
        app,
        Message::Window(
            id,
            WindowEvent::Key(typing_event(Key::Character(text.as_str().into()), Modifiers::empty(), &text)),
        ),
    );
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
    send(&mut app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(Named::End), Modifiers::empty()))));
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
    insert_link(&mut app, id);
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
    editor(&mut app, id, EditorMsg::Command(EditorCommand::SelectWord));
    assert_eq!(app.editors[&id].page.selected_source(), "more");
    press(&mut app, id, "l", Modifiers::CTRL);
    assert_eq!(app.editors[&id].page.selected_source(), "some **bold text** and more");
}

#[test]
fn ctrl_click_on_an_anchor_goes_to_the_heading() {
    let (mut app, _f, _, _) = app_with_page("[to b](#part-b)\n\n# Part A\n\n# Part B\n");
    let id = open_with_f1(&mut app);
    send(&mut app, Message::Window(id, WindowEvent::Key(keyboard::Event::ModifiersChanged(Modifiers::CTRL))));
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
    send(&mut app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(Named::Delete), Modifiers::empty()))));
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
    send(&mut app, Message::Window(id, WindowEvent::Key(keyboard::Event::ModifiersChanged(Modifiers::SHIFT))));
    editor(&mut app, id, EditorMsg::BlockPressed(1));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((0, 1)));
    send(&mut app, Message::Window(id, WindowEvent::Key(keyboard::Event::ModifiersChanged(Modifiers::empty()))));
    // Typing replaces them.
    send(
        &mut app,
        Message::Window(id, WindowEvent::Key(typing_event(Key::Character("z".into()), Modifiers::empty(), "z"))),
    );
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

/// The counts are kept between views, and counted again after every change
/// of the page or of the selection.
#[test]
fn the_counts_follow_edits_and_the_selection() {
    let (mut app, _f, _, _) = app_with_page("one two\n");
    let id = open_with_f1(&mut app);
    let words = |app: &App| {
        let (all, selected) = app.editors[&id].counts();
        (all.words, selected.map(|s| s.words))
    };
    assert_eq!(words(&app), (2, None));
    click_after(&mut app, id, "two");
    typing(&mut app, id, " three");
    assert_eq!(words(&app), (3, None));
    select(&mut app, id, "one");
    assert_eq!(words(&app), (3, Some(1)));
    select(&mut app, id, "two three");
    assert_eq!(words(&app), (3, Some(2)));
    // The selection's text changed, the selection did not.
    typing(&mut app, id, "four");
    assert_eq!(words(&app), (2, None));
    press(&mut app, id, "z", Modifiers::CTRL);
    assert_eq!(words(&app).0, 3);
}

#[test]
fn deleting_three_blocks() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n\nfour\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    // From the text editor's last line to the next block; then the window's keys.
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    send(&mut app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(Named::ArrowDown), Modifiers::SHIFT))));
    assert_eq!(app.editors[&id].page.selected_blocks(), Some((0, 2)));
    send(&mut app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(Named::Delete), Modifiers::empty()))));
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

#[test]
fn enter_after_an_items_text_keeps_its_code_block() {
    let (mut app, _f, _, _) = app_with_page("* aaa\n\n  \n  ```json\n  aaa\n  ```\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "* aaa");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    typing(&mut app, id, "b");
    // The list needs no blank lines before the code block: it is written tight.
    assert_eq!(page_text(&app, id), "* aaa\n  ```json\n  aaa\n  ```\n* b\n");
    assert_eq!(app.editors[&id].page.region_text(), Some("* b"));
}

#[test]
fn down_below_code_ending_a_list_item_adds_a_paragraph_in_it() {
    use crate::markdown::BlockKind;
    let (mut app, _f, _, _) = app_with_page("* aaa\n\n  ```json\n  aaa\n  ```\n\nnext\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "json\n  aa");
    editor(&mut app, id, EditorMsg::Bind(Bind::Down));
    assert!(app.editors[&id].page.pending().is_some());
    typing(&mut app, id, "z");
    assert_eq!(page_text(&app, id), "* aaa\n\n  ```json\n  aaa\n  ```\n\n  z\n\nnext\n");
    // Down again from an empty one: into the block after the item.
    let (mut app, _f, _, _) = app_with_page("* aaa\n\n  ```json\n  aaa\n  ```\n\nnext\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "json\n  aa");
    editor(&mut app, id, EditorMsg::Bind(Bind::Down));
    editor(&mut app, id, EditorMsg::Bind(Bind::Down));
    assert_eq!(app.editors[&id].page.region_text(), Some("next"));
    assert_eq!(page_text(&app, id), "* aaa\n\n  ```json\n  aaa\n  ```\n\nnext\n");
    // Code followed by another block of the item: into that block.
    let (mut app, _f, _, _) = app_with_page("* aaa\n\n  ```\n  x\n  ```\n\n  more\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "```\n  x");
    assert!(matches!(active_kind(&app, id), BlockKind::Code { .. }));
    editor(&mut app, id, EditorMsg::Bind(Bind::Down));
    assert_eq!(app.editors[&id].page.region_text(), Some("  more"));
}

#[test]
fn a_press_is_held_for_the_editor_that_takes_the_block_s_place() {
    let (mut app, _f, _, _) = app_with_page("one\n\n```\ncode\n```\n");
    let id = open_with_f1(&mut app);
    let pad = iced::Padding::ZERO;
    assert!(app.editors[&id].held(pad).is_none());
    // Pressing the rendered code block, the point where it went down is held until the release.
    editor(&mut app, id, EditorMsg::BlockPressed(1));
    editor(&mut app, id, EditorMsg::PressPoint(iced::Point::new(30.0, 40.0)));
    let held = app.editors[&id].held(pad).expect("a held press");
    assert_eq!((held.from.x, held.from.y), (30.0, 40.0));
    editor(&mut app, id, EditorMsg::Released);
    assert!(app.editors[&id].held(pad).is_none());
}

/// The page after typing "bbb " into the heading the cursor was moved to with `go`.
fn typed_into_heading(go: impl FnOnce(&mut App, window::Id)) -> String {
    let (mut app, _f, _, _) = app_with_page("before\n\n### aaa\n\nafter\n");
    let id = open_with_f1(&mut app);
    go(&mut app, id);
    typing(&mut app, id, "bbb ");
    page_text(&app, id)
}

fn bind(app: &mut App, id: window::Id, bind: Bind) {
    editor(app, id, EditorMsg::Bind(bind));
}

fn home(app: &mut App, id: window::Id) {
    editor(app, id, EditorMsg::Action(Action::Move(iced::widget::text_editor::Motion::Home)));
}

#[test]
fn typing_at_the_start_of_a_heading_goes_after_its_marker() {
    let expected = "before\n\n### bbb aaa\n\nafter\n";
    // Right arrow at the end of the block before.
    assert_eq!(
        typed_into_heading(|app, id| {
            click_after(app, id, "before");
            bind(app, id, Bind::Right);
        }),
        expected
    );
    // Down from the block before, at its start.
    assert_eq!(
        typed_into_heading(|app, id| {
            click_after(app, id, "before");
            home(app, id);
            bind(app, id, Bind::Down);
        }),
        expected
    );
    // Up from the block below, at its start.
    assert_eq!(
        typed_into_heading(|app, id| {
            click_after(app, id, "after");
            home(app, id);
            bind(app, id, Bind::Up);
        }),
        expected
    );
    // A press beside the heading's text.
    assert_eq!(typed_into_heading(|app, id| editor(app, id, EditorMsg::BlockPressed(1))), expected);
    // Selected blocks left with Up.
    assert_eq!(
        typed_into_heading(|app, id| {
            editor(app, id, EditorMsg::BlockPressed(1));
            app.editors.get_mut(&id).unwrap().page.select_blocks(1, 1);
            send(app, Message::Window(id, WindowEvent::Key(key_event(Key::Named(Named::ArrowUp), Modifiers::empty()))));
        }),
        expected
    );
}

#[test]
fn enter_at_a_heading_start_keeps_the_caret_in_the_heading() {
    for (before, after) in [
        ("## Plan\n", "\n\n## Plan\n"),
        ("Before\n\n## Plan\n", "Before\n\n\n\n## Plan\n"),
        ("> ## Plan\n", ">\n>\n> ## Plan\n"),
        ("* List\n  ## Plan\n* next\n", "* List\n\n\n  ## Plan\n\n* next\n"),
    ] {
        let (mut app, _f, _, _) = app_with_page(before);
        let id = open_with_f1(&mut app);
        click_after(&mut app, id, "## ");
        bind(&mut app, id, Bind::Enter);
        let page = &app.editors[&id].page;
        assert_eq!(page.text(), after);
        assert_eq!(page.sel().head, after.find("Plan").unwrap());
        assert!(matches!(active_kind(&app, id), crate::markdown::BlockKind::Heading(2)));

        editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
        let page = &app.editors[&id].page;
        assert_eq!(page.text(), before);
        assert_eq!(page.sel().head, before.find("Plan").unwrap());

        editor(&mut app, id, EditorMsg::Command(EditorCommand::Redo));
        type_keys(&mut app, id, "New ");
        assert_eq!(page_text(&app, id), after.replace("Plan", "New Plan"));
    }
}

#[test]
fn an_empty_paragraph_above_a_heading_can_be_entered_and_typed_into() {
    for (source, expected) in [
        ("## Plan\n", "Intro\n\n## Plan\n"),
        ("> ## Plan\n", "> Intro\n>\n> ## Plan\n"),
        ("* List\n  ## Plan\n* next\n", "* List\n\n  Intro\n\n  ## Plan\n\n* next\n"),
    ] {
        let (mut app, _f, _, _) = app_with_page(source);
        let id = open_with_f1(&mut app);
        click_after(&mut app, id, "## ");
        bind(&mut app, id, Bind::Enter);
        bind(&mut app, id, Bind::Up);
        assert!(app.editors[&id].page.pending().is_some());
        bind(&mut app, id, Bind::Down);
        assert!(matches!(active_kind(&app, id), crate::markdown::BlockKind::Heading(2)));
        bind(&mut app, id, Bind::Up);
        type_keys(&mut app, id, "Intro");
        assert_eq!(page_text(&app, id), expected);
    }
}

#[test]
fn clicking_an_empty_paragraph_replaces_its_row_with_the_active_editor() {
    use super::super::column::{Shown, entries};

    let (mut app, _f, _, _) = app_with_page("\n\n\n\n## Plan\n");
    let id = open_with_f1(&mut app);
    let shown = |app: &App| entries(&app.editors[&id].page).into_iter().map(|e| e.shown).collect::<Vec<_>>();
    let first = Shown::Blank { before: 0, at: 0 };
    let second = Shown::Blank { before: 0, at: 2 };
    assert_eq!(shown(&app), [first, second, Shown::Block(0)]);
    editor(&mut app, id, EditorMsg::PressedAt(2));
    assert_eq!(shown(&app), [first, Shown::Active, Shown::Block(0)]);
    bind(&mut app, id, Bind::Up);
    assert_eq!(shown(&app), [Shown::Active, second, Shown::Block(0)]);
    type_keys(&mut app, id, "Intro");
    assert_eq!(page_text(&app, id), "Intro\n\n\n\n## Plan\n");
}

#[test]
fn deleting_a_displayed_empty_paragraph_preserves_the_block_separator() {
    use super::super::column::{Shown, entries};

    for (source, expected, previous, next) in [
        ("one\n\n\ntwo\n", "one\n\ntwo\n", "one", "two"),
        ("one\n\n\n\ntwo\n", "one\n\ntwo\n", "one", "two"),
        ("> one\n>\n> \t\n> two\n", "> one\n> \t\n> two\n", "one", "two"),
        (
            "* Item\n\n\n  Paragraph\n\n* next\n* final\n",
            "* Item\n\n  Paragraph\n\n* next\n* final\n",
            "Item",
            "Paragraph",
        ),
    ] {
        for key in [Bind::Backspace, Bind::Delete] {
            let (mut app, _f, _, _) = app_with_page(source);
            let id = open_with_f1(&mut app);
            let at = entries(&app.editors[&id].page)
                .iter()
                .find_map(|entry| match entry.shown {
                    Shown::Blank { at, .. } => Some(at),
                    _ => None,
                })
                .expect("a displayed empty paragraph");
            let caret = if key == Bind::Backspace {
                expected.find(previous).unwrap() + previous.len()
            } else {
                expected.find(next).unwrap()
            };
            editor(&mut app, id, EditorMsg::PressedAt(at));
            bind(&mut app, id, key);
            let page = &app.editors[&id].page;
            assert_eq!(page.text(), expected);
            assert_eq!(page.sel().head, caret);
            assert!(entries(page).iter().all(|entry| !matches!(entry.shown, Shown::Blank { .. })));

            editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
            let page = &app.editors[&id].page;
            assert_eq!(page.text(), source);
            assert_eq!(page.sel().head, at);
            assert!(page.pending().is_some());

            editor(&mut app, id, EditorMsg::Command(EditorCommand::Redo));
            assert_eq!(page_text(&app, id), expected);
            assert_eq!(app.editors[&id].page.sel().head, caret);
        }
    }
}

#[test]
fn deleting_one_of_several_leading_empty_paragraphs_survives_reopening() {
    use super::super::column::{Shown, entries};
    use super::super::page::Page;

    let (mut app, _f, _, path) = app_with_page("\n \t\n\n\n## Plan\n");
    std::fs::write(&path, "\u{feff}\r\n \t\r\n\r\n\r\n## Plan\r\n").unwrap();
    let id = open_with_f1(&mut app);
    editor(&mut app, id, EditorMsg::BlockPressed(0));
    bind(&mut app, id, Bind::Up);
    assert_eq!(app.editors[&id].page.sel().head, 4);
    bind(&mut app, id, Bind::Backspace);
    assert_eq!(page_text(&app, id), "\n \t\n## Plan\n");
    assert_eq!(app.editors[&id].page.sel().head, 7);
    editor(&mut app, id, EditorMsg::Command(EditorCommand::Save));

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "\u{feff}\r\n \t\r\n## Plan\r\n");
    let reopened = Page::open(&path).unwrap();
    let shown = entries(&reopened).into_iter().map(|entry| entry.shown).collect::<Vec<_>>();
    assert_eq!(shown, [Shown::Blank { before: 0, at: 0 }, Shown::Block(0)]);
}

#[test]
fn deleting_an_unwritten_paragraph_leaves_the_source_intact() {
    for key in [Bind::Backspace, Bind::Delete] {
        let source = "## Plan\n";
        let (mut app, _f, _, _) = app_with_page(source);
        let id = open_with_f1(&mut app);
        click_after(&mut app, id, "Plan");
        bind(&mut app, id, Bind::Enter);
        assert!(app.editors[&id].page.pending().is_some());
        bind(&mut app, id, key);
        assert_eq!(page_text(&app, id), source);
    }
}

#[test]
fn enter_at_the_start_of_a_paragraph_keeps_the_caret_with_its_text() {
    for (before, after, with_intro) in [
        ("Paragraph\n", "\n\nParagraph\n", "Intro\n\nParagraph\n"),
        ("> Paragraph\n", ">\n>\n> Paragraph\n", "> Intro\n>\n> Paragraph\n"),
        ("* Item\n\n  Paragraph\n", "* Item\n\n\n\n  Paragraph\n", "* Item\n\n  Intro\n\n  Paragraph\n"),
    ] {
        let (mut app, _f, _, _) = app_with_page(before);
        let id = open_with_f1(&mut app);
        editor(&mut app, id, EditorMsg::PressedAt(before.find("Paragraph").unwrap()));
        bind(&mut app, id, Bind::Enter);
        assert_eq!(page_text(&app, id), after);
        assert_eq!(app.editors[&id].page.sel().head, after.find("Paragraph").unwrap());
        bind(&mut app, id, Bind::Up);
        assert!(app.editors[&id].page.pending().is_some());
        type_keys(&mut app, id, "Intro");
        assert_eq!(page_text(&app, id), with_intro);
    }
}

#[test]
fn splitting_a_list_item_keeps_its_check_or_updates_its_number() {
    for (before, after) in [
        ("- [x] Done\n- [ ] Next\n", "- [ ] \n- [x] Done\n- [ ] Next\n"),
        ("3. Done\n3. Next\n", "3. \n4. Done\n5. Next\n"),
    ] {
        let (mut app, _f, _, _) = app_with_page(before);
        let id = open_with_f1(&mut app);
        editor(&mut app, id, EditorMsg::PressedAt(before.find("Done").unwrap()));
        bind(&mut app, id, Bind::Enter);
        assert_eq!(page_text(&app, id), after);
        assert_eq!(app.editors[&id].page.sel().head, after.find("Done").unwrap());
        editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
        assert_eq!(page_text(&app, id), before);
    }
}

#[test]
fn enter_at_the_start_of_indented_code_keeps_a_visible_blank_line() {
    use crate::markdown::edit::CodeBody;

    for indent in ["    ", "\t"] {
        let before = format!("{indent}first\n{indent}next\n\nAfter\n");
        let after = format!("{indent}\n{indent}first\n{indent}next\n\nAfter\n");
        let (mut app, _f, _, file) = app_with_page(&before);
        let id = open_with_f1(&mut app);
        editor(&mut app, id, EditorMsg::PressedAt(indent.len()));
        bind(&mut app, id, Bind::Enter);
        assert_eq!(page_text(&app, id), after);
        let page = &app.editors[&id].page;
        assert_eq!(page.active().unwrap().code().unwrap().text(), "\nfirst\nnext");
        assert_eq!(page.sel().head, after.find("first").unwrap());

        editor(&mut app, id, EditorMsg::Command(EditorCommand::Save));
        let saved = crate::markdown::page::PageText::read(&file).unwrap();
        let doc = crate::markdown::Document::parse(&saved.text);
        assert_eq!(CodeBody::of(&doc.text, &doc, 0).unwrap().text(), "\nfirst\nnext");
    }
}

#[test]
fn enter_inside_the_opening_of_front_matter_or_math_keeps_the_block() {
    use crate::markdown::BlockKind;

    for (kind, source, off) in [
        (BlockKind::FrontMatter, "---\ntitle: Plan\n---\n\nAfter\n", 4),
        (BlockKind::Math, "$$\nx^2\n$$\n\nAfter\n", 3),
    ] {
        for start in [1, off] {
            let (mut app, _f, _, _) = app_with_page(source);
            let id = open_with_f1(&mut app);
            editor(&mut app, id, EditorMsg::PressedAt(start));
            bind(&mut app, id, Bind::Enter);
            let text = format!("{}\n{}", &source[..off], &source[off..]);
            assert_eq!(page_text(&app, id), text);
            let page = &app.editors[&id].page;
            assert_eq!(page.active().unwrap().kind, kind);
            assert_eq!(page.sel().head, off + 1);
            assert_eq!(crate::markdown::Document::parse(&text).blocks[0].kind, kind);
            editor(&mut app, id, EditorMsg::Command(EditorCommand::Undo));
            assert_eq!(page_text(&app, id), source);
        }
    }
}

#[test]
fn a_paragraph_typed_into_a_math_block_keeps_the_cursor() {
    let (mut app, _f, _, _) = app_with_page("Intro\n\n$$\n\\frac{1}{0\n$$\n");
    let id = open_with_f1(&mut app);
    // Unbalanced, `$$` ... `$$` is a paragraph: clicked into, the cursor to its line's end.
    let at = page_text(&app, id).find("\\f").unwrap() + 2;
    editor(&mut app, id, EditorMsg::PressedAt(at));
    live(&mut app, id, LiveMsg::MoveTo { at: "$$\n\\frac{1}{0".len(), select: false });
    // `}` makes it a math block, edited as its source; typing goes on where the cursor was.
    type_keys(&mut app, id, "}");
    typing(&mut app, id, "X");
    assert_eq!(page_text(&app, id), "Intro\n\n$$\n\\frac{1}{0}X\n$$\n");
}
