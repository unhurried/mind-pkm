//! Word and character counts of what a page shows (Markdown markers left out).

use super::content::{self, Inline, Node};
use super::parse::{BlockKind, Document};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub words: usize,
    pub characters: usize,
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}'
        | '\u{ff66}'..='\u{ff9f}' | '\u{ac00}'..='\u{d7af}')
}

/// Counts of a shown text: each CJK character is a word, other words are runs
/// of letters and digits; characters are all but line breaks.
pub fn count(shown: &str) -> Counts {
    let mut words = 0;
    let mut in_word = false;
    for c in shown.chars() {
        if is_cjk(c) {
            words += 1;
            in_word = false;
        } else if c.is_alphanumeric() || c == '_' || (in_word && (c == '\'' || c == '’')) {
            if !in_word {
                words += 1;
            }
            in_word = true;
        } else {
            in_word = false;
        }
    }
    let characters = shown.chars().filter(|c| *c != '\n' && *c != '\r').count();
    Counts { words, characters }
}

/// The text a page shows, block by block.
pub fn shown_text(doc: &Document) -> String {
    let mut out = String::new();
    for (i, block) in doc.blocks.iter().enumerate() {
        match &block.kind {
            BlockKind::FrontMatter | BlockKind::Other => continue,
            BlockKind::Table => {
                let t = content::table_block(doc, i);
                for row in std::iter::once(&t.header).chain(&t.rows) {
                    for cell in row {
                        out.push_str(&content::shown_text(cell));
                        out.push(' ');
                    }
                }
            }
            _ => nodes_text(&content::nodes(doc, doc.block_events(i)), &mut out),
        }
        out.push('\n');
    }
    out
}

fn nodes_text(nodes: &[Node], out: &mut String) {
    for n in nodes {
        match n {
            Node::Paragraph(i) | Node::Heading(_, i) | Node::Html(i) => inline_text(i, out),
            Node::Code { text, .. } | Node::Math(text) => out.push_str(text),
            Node::Quote(c) | Node::Footnote { children: c, .. } => nodes_text(c, out),
            Node::List { items, .. } => items.iter().for_each(|it| nodes_text(&it.children, out)),
            Node::Table(t) => {
                for row in std::iter::once(&t.header).chain(&t.rows) {
                    row.iter().for_each(|c| inline_text(c, out));
                }
            }
            Node::Rule | Node::FrontMatter(_) => {}
        }
        out.push('\n');
    }
}

fn inline_text(inlines: &[Inline], out: &mut String) {
    out.push_str(&content::shown_text(inlines));
}

/// Counts of a piece of Markdown source (a selection).
pub fn count_source(source: &str) -> Counts {
    count(&shown_text(&Document::parse(source)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_characters() {
        assert_eq!(count("two words"), Counts { words: 2, characters: 9 });
        assert_eq!(count("日本語 and English"), Counts { words: 5, characters: 15 });
        assert_eq!(count("don't stop"), Counts { words: 2, characters: 10 });
        assert_eq!(count("a\nb"), Counts { words: 2, characters: 2 });
    }

    #[test]
    fn markers_are_not_counted() {
        assert_eq!(count_source("# Title\n\n- **two** words\n"), Counts { words: 3, characters: 14 });
        assert_eq!(count_source("---\nx: y\n---\n\n[a](http://x.y)"), Counts { words: 1, characters: 1 });
    }
}
