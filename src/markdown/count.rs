//! Word and character counts of what a page shows (Markdown markers left out).

use super::content::{self, Inline, Node};
use super::parse::{BlockKind, Document};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

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
    for i in 0..doc.blocks.len() {
        block_text(doc, i, &mut out);
        out.push('\n');
    }
    out
}

/// The text the block `i` of `doc` shows.
fn block_text(doc: &Document, i: usize, out: &mut String) {
    match &doc.blocks[i].kind {
        BlockKind::FrontMatter | BlockKind::Other => {}
        BlockKind::Table => {
            let t = content::table_block(doc, i);
            for row in std::iter::once(&t.header).chain(&t.rows) {
                for cell in row {
                    out.push_str(&content::shown_text(cell));
                    out.push(' ');
                }
            }
        }
        _ => nodes_text(&content::nodes(doc, doc.block_events(i)), out),
    }
}

/// The counts of the blocks of pages, kept by what decides them: a block's
/// source and its parse (which other blocks change too: a link reference
/// defined elsewhere makes a link of its text).
#[derive(Debug, Default)]
pub struct BlockCounts {
    of: HashMap<u64, Counts>,
}

impl BlockCounts {
    /// `count(&shown_text(doc))`, counting again only the blocks whose
    /// source or parse changed since: each block's text is followed by a
    /// line break, which ends a word and is no character, so a page's counts
    /// are the sums of its blocks'.
    pub fn count(&mut self, doc: &Document) -> Counts {
        let mut total = Counts::default();
        let mut seen = HashMap::with_capacity(doc.blocks.len());
        for i in 0..doc.blocks.len() {
            let key = block_key(doc, i);
            let c = match self.of.get(&key) {
                Some(&c) => c,
                None => {
                    let mut text = String::new();
                    block_text(doc, i, &mut text);
                    count(&text)
                }
            };
            seen.insert(key, c);
            total.words += c.words;
            total.characters += c.characters;
        }
        self.of = seen;
        total
    }
}

/// What the counts of the block `i` of `doc` depend on: its kind, its source
/// and its parse (each event's kind and place in the block).
fn block_key(doc: &Document, i: usize) -> u64 {
    let block = &doc.blocks[i];
    let mut hasher = DefaultHasher::new();
    std::mem::discriminant(&block.kind).hash(&mut hasher);
    doc.text[block.range.clone()].hash(&mut hasher);
    for (event, range) in doc.block_events(i) {
        std::mem::discriminant(event).hash(&mut hasher);
        range.start.wrapping_sub(block.range.start).hash(&mut hasher);
        range.end.wrapping_sub(block.range.start).hash(&mut hasher);
    }
    hasher.finish()
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

    /// A page's counts block by block are its counts as a whole, also as
    /// blocks change and come and go.
    #[test]
    fn counted_block_by_block_as_a_whole() {
        let mut counts = BlockCounts::default();
        let pages = [
            "# Title\n\nSome *words* here.\n\n- a list `item`\n- 日本語の項目\n\n| a | b |\n| --- | --- |\n| c d | e |\n",
            "# Title\n\nSome *words* here, more.\n\n- a list `item`\n- 日本語の項目\n\n| a | b |\n| --- | --- |\n| c d | e |\n",
            "---\nx: y\n---\n\n# Title\n\n> quoted $x^2$ text\n\n```rust\nlet x = 1;\n```\n\nSome *words* here, more.\n",
            // A link reference defined elsewhere changes how a block counts.
            "See [the docs][d] now.\n",
            "See [the docs][d] now.\n\n[d]: https://example.com\n",
            "",
        ];
        for text in pages {
            let doc = Document::parse(text);
            assert_eq!(counts.count(&doc), count(&shown_text(&doc)), "{text:?}");
        }
        assert_ne!(count_source(pages[3]), count_source(pages[4]));
    }
}
