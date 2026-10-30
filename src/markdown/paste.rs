//! What pasting puts into a page, independent of the UI: HTML converted to
//! Markdown, a table (from a spreadsheet or a web page) as cells, other text
//! as it is. Pasted files and images are the editor's to insert.

use super::html::to_plain_text;
use super::table::{self, HtmlPart};

/// What a paste puts into the page.
#[derive(Debug, Clone, PartialEq)]
pub enum Paste {
    /// Cells, the first row the header: into the table being edited, else a new table.
    Cells(Vec<Vec<String>>),
    /// Text inserted at the cursor: Markdown converted from HTML, or the clipboard's text.
    Text(String),
    Nothing,
}

/// The clipboard's text with LF line endings. In code (a code block, math,
/// ...) with no text flavor: what its HTML says, since code is code.
pub fn clipboard_text(text: Option<&str>, html: Option<&str>, in_code: bool) -> String {
    let text = text.unwrap_or_default().replace("\r\n", "\n");
    match html {
        Some(html) if in_code && text.is_empty() => to_plain_text(html).replace("\r\n", "\n"),
        _ => text,
    }
}

/// What pasting `text` (see [`clipboard_text`]) and `html` puts into the page.
/// The text goes in as it is when pasted as plain text (`plain`), into code
/// (`in_code`, never the HTML converted), or when it is what this editor last
/// copied (`copied`).
pub fn convert(text: &str, html: Option<&str>, in_code: bool, plain: bool, copied: Option<&str>) -> Paste {
    if !plain
        && !in_code
        && (copied != Some(text) || text.is_empty())
        && let Some(paste) = converted(text, html)
    {
        return paste;
    }
    if text.is_empty() { Paste::Nothing } else { Paste::Text(text.to_string()) }
}

/// A table (from a spreadsheet or a web page), alone or with other content
/// around it, or the HTML as Markdown; None when neither gives anything.
fn converted(text: &str, html: Option<&str>) -> Option<Paste> {
    let parts =
        html.map(|h| table::split_html_tables(h, |cell| htmd::convert(cell).unwrap_or_default())).unwrap_or_default();
    let has_table = parts.iter().any(|p| matches!(p, HtmlPart::Table(_)));
    let content: Vec<_> =
        parts.iter().filter(|p| !matches!(p, HtmlPart::Html(h) if to_plain_text(h).trim().is_empty())).collect();
    let grid = match content.as_slice() {
        [HtmlPart::Table(grid)] => Some(grid.clone()),
        _ if has_table => None,
        _ => table::parse_tsv(text),
    }
    .filter(|g| g.len() > 1 || g.first().is_some_and(|r| r.len() > 1));
    if let Some(grid) = grid {
        return Some(Paste::Cells(grid));
    }
    let markdown = if has_table {
        let pieces: Vec<String> = content
            .iter()
            .map(|p| match p {
                HtmlPart::Html(h) => htmd::convert(h).unwrap_or_default().trim().to_string(),
                HtmlPart::Table(grid) => cells_markdown(grid),
            })
            .filter(|m| !m.is_empty())
            .collect();
        Some(pieces.join("\n\n"))
    } else {
        html.and_then(|h| htmd::convert(h).ok())
    };
    markdown.map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).map(Paste::Text)
}

/// Pasted cells as a Markdown table, the first row its header.
pub fn cells_markdown(grid: &[Vec<String>]) -> String {
    let mut t = table::Table::new(grid.len().saturating_sub(1), grid.iter().map(Vec::len).max().unwrap_or(1));
    t.paste((0, 0), grid);
    t.to_markdown()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Paste {
        Paste::Text(s.to_string())
    }

    #[test]
    fn html_becomes_markdown_unless_pasted_as_plain_text() {
        let html = "<b>bold</b> and <a href=\"https://x.org\">link</a>";
        assert_eq!(
            convert("bold and link", Some(html), false, false, None),
            text("**bold** and [link](https://x.org)")
        );
        assert_eq!(convert("bold and link", Some(html), false, true, None), text("bold and link"));
        assert_eq!(convert("", None, false, false, None), Paste::Nothing);
    }

    #[test]
    fn code_takes_text_as_it_is() {
        // What VS Code puts on the clipboard.
        let html = "<div><div><span>let a_b = c*d;</span></div><div><span>x &lt; y</span></div></div>";
        assert_eq!(clipboard_text(Some("a\r\nb"), Some(html), true), "a\nb");
        assert_eq!(clipboard_text(None, Some(html), true), "let a_b = c*d;\nx < y");
        assert_eq!(clipboard_text(None, Some(html), false), "");
        assert_eq!(convert("a\tb\n1\t2", Some(html), true, false, None), text("a\tb\n1\t2"));
    }

    #[test]
    fn tab_separated_rows_are_cells() {
        let cells = convert("a\tb\n1\t2\n", None, false, false, None);
        let grid = vec![vec!["a".to_string(), "b".to_string()], vec!["1".to_string(), "2".to_string()]];
        assert_eq!(cells, Paste::Cells(grid.clone()));
        assert_eq!(cells_markdown(&grid), "| a   | b   |\n| --- | --- |\n| 1   | 2   |");
        // Indented text is no table; what this editor copied goes back as it was.
        assert_eq!(convert("def g():\n\treturn 1", None, false, false, None), text("def g():\n\treturn 1"));
        assert_eq!(convert("a\tb\n1\t2", None, false, false, Some("a\tb\n1\t2")), text("a\tb\n1\t2"));
    }

    #[test]
    fn html_around_a_table_is_kept() {
        let html = "<p>Hello <b>bold</b> and <a href=\"https://example.com\">site</a></p>\n\
                    <ul><li>one</li><li>two</li></ul>\n\
                    <table><tr><th>H1</th><th>H2</th></tr><tr><td>c1</td><td>c2</td></tr></table>";
        let pasted = convert("Hello bold and site\none\ntwo\nH1\tH2\nc1\tc2", Some(html), false, false, None);
        assert_eq!(
            pasted,
            text(
                "Hello **bold** and [site](https://example.com)\n\n*   one\n*   two\n\n| H1  | H2  |\n| --- | --- |\n| c1  | c2  |"
            )
        );
        // A table alone (with the head a spreadsheet adds) is cells.
        let sheet = "<html><head><style>td { color: red }</style></head><body><table><tr><td>a</td><td>b</td></tr></table></body></html>";
        assert_eq!(convert("a\tb", Some(sheet), false, false, None), Paste::Cells(vec![vec!["a".into(), "b".into()]]));
    }
}
