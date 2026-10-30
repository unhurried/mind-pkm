//! Copying, cutting and pasting, and inserting files.

use super::*;

#[test]
fn copy_puts_markdown_and_html_on_the_clipboard() {
    let (mut app, _f, _, _) = app_with_page("a **bold** word\n");
    let id = open_with_f1(&mut app);
    select(&mut app, id, "**bold** word");
    press(&mut app, id, "c", Modifiers::CTRL);
    let c = clipboard(&app);
    assert_eq!(c.text.as_deref(), Some("**bold** word"));
    assert!(c.html.as_deref().is_some_and(|h| h.contains("<strong>bold</strong>")), "{c:?}");
    // Copy as Markdown: no HTML.
    press(&mut app, id, "c", Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(clipboard(&app).html, None);
    // Pasted back here: exactly the Markdown.
    click_after(&mut app, id, "word");
    pasted(&mut app, id, "**bold** word", Some("<strong>bold</strong> word"), false);
    assert_eq!(page_text(&app, id), "a **bold** word**bold** word\n");
}

#[test]
fn html_from_other_applications_is_pasted_as_markdown() {
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    pasted(&mut app, id, "bold and link", Some("<b>bold</b> and <a href=\"https://x.org\">link</a>"), false);
    assert_eq!(page_text(&app, id), "x**bold** and [link](https://x.org)\n");
    // Paste as plain text keeps the text.
    pasted(&mut app, id, " plain", Some("<b>plain</b>"), true);
    assert_eq!(page_text(&app, id), "x**bold** and [link](https://x.org) plain\n");
}

#[test]
fn html_pasted_into_a_code_block_is_inserted_as_plain_text() {
    // What VS Code puts on the clipboard.
    let html = "<div><div><span>let a_b = c*d;</span></div><div><span>x &lt; y</span></div></div>";
    let (mut app, _f, _, _) = app_with_page("```\nfoo\n```\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "foo");
    // Both flavors: the text one.
    pasted(&mut app, id, "let a_b = c*d;\nx < y", Some(html), false);
    assert_eq!(page_text(&app, id), "```\nfoolet a_b = c*d;\nx < y\n```\n");
    // The HTML flavor only: its text.
    let content = crate::app::editor::clipboard::ClipboardContent { html: Some(html.into()), ..Default::default() };
    editor(&mut app, id, EditorMsg::Pasted(content, false));
    assert_eq!(page_text(&app, id), "```\nfoolet a_b = c*d;\nx < ylet a_b = c*d;\nx < y\n```\n");
}

#[test]
fn a_spreadsheet_range_pasted_outside_a_table_becomes_a_table() {
    let (mut app, _f, _, _) = app_with_page("before\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "before");
    pasted(&mut app, id, "a\tb\n1\t2\n", None, false);
    assert_eq!(page_text(&app, id), "before\n\n| a   | b   |\n| --- | --- |\n| 1   | 2   |\n");
}

#[test]
fn tab_indented_text_is_pasted_as_text() {
    let (mut app, _f, _, _) = app_with_page("x\n\n```\nfoo\n```\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    pasted(&mut app, id, "def g():\n\treturn 1", None, false);
    assert!(!page_text(&app, id).contains('|'), "{}", page_text(&app, id));
    assert!(page_text(&app, id).starts_with("xdef g():\n"), "{}", page_text(&app, id));
    // In a code block: verbatim, even a grid.
    click_after(&mut app, id, "foo");
    pasted(&mut app, id, "a\tb\n1\t2", None, false);
    assert!(page_text(&app, id).ends_with("```\nfooa\tb\n1\t2\n```\n"), "{}", page_text(&app, id));
}

#[test]
fn html_around_a_table_is_kept() {
    let html = "<p>Hello <b>bold</b> and <a href=\"https://example.com\">site</a></p>\n\
                <ul><li>one</li><li>two</li></ul>\n\
                <table><tr><th>H1</th><th>H2</th></tr><tr><td>c1</td><td>c2</td></tr></table>";
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    select(&mut app, id, "x");
    pasted(&mut app, id, "Hello bold and site\none\ntwo\nH1\tH2\nc1\tc2", Some(html), false);
    assert_eq!(
        page_text(&app, id).trim_end(),
        "Hello **bold** and [site](https://example.com)\n\n*   one\n*   two\n\n| H1  | H2  |\n| --- | --- |\n| c1  | c2  |"
    );
    // A table alone (with the head a spreadsheet adds) is still a table.
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    select(&mut app, id, "x");
    let sheet = "<html><head><style>td { color: red }</style></head><body><table><tr><td>a</td><td>b</td></tr></table></body></html>";
    pasted(&mut app, id, "a\tb", Some(sheet), false);
    assert_eq!(page_text(&app, id).trim(), "x\n\n| a   | b   |\n| --- | --- |\n|     |     |");
}

#[test]
fn cutting_whole_blocks() {
    let (mut app, _f, _, _) = app_with_page("one\n\ntwo\n\nthree\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "one");
    editor(&mut app, id, EditorMsg::Bind(Bind::ShiftDown));
    assert_eq!(app.editors[&id].page.blocks, Some((0, 1)));
    press(&mut app, id, "x", Modifiers::CTRL);
    assert_eq!(clipboard(&app).text.as_deref(), Some("one\n\ntwo"));
    assert_eq!(page_text(&app, id), "three\n");
}

#[test]
fn inserted_images_go_one_per_line() {
    let (mut app, _f, _, _) = app_with_page("x\n");
    let id = open_with_f1(&mut app);
    click_after(&mut app, id, "x");
    editor(&mut app, id, EditorMsg::Bind(Bind::Enter));
    editor(
        &mut app,
        id,
        EditorMsg::Inserted(Ok(vec!["![](Plan.assets/a.png)".into(), "![](Plan.assets/b.jpg)".into()])),
    );
    assert_eq!(page_text(&app, id), "x\n\n![](Plan.assets/a.png)\n![](Plan.assets/b.jpg)\n");
    // Errors are shown.
    editor(&mut app, id, EditorMsg::Inserted(Err("failed to copy x".into())));
    assert_eq!(app.editors[&id].dialogs.front(), Some(&EditorDialog::Message("failed to copy x".into())));
}

#[test]
fn dropped_files_are_copied_into_the_assets_folder() {
    let (_app, f, _, page) = app_with_page("x\n");
    let picture = f.dir.path().join("photo.png");
    std::fs::write(&picture, b"png").unwrap();
    let pdf = f.dir.path().join("doc.pdf");
    std::fs::write(&pdf, b"pdf").unwrap();
    let opts = crate::markdown::images::ImageOptions::default();
    let md = crate::markdown::images::files_markdown(&page, "x\n", &[picture, pdf], &opts).unwrap();
    assert_eq!(md, vec!["![](Plan.assets/photo.png)".to_string(), "[doc.pdf](../doc.pdf)".to_string()]);
    assert!(f.doc_dir.join("Plan.assets").join("photo.png").is_file());
}
