//! Property tests of the page editor's Markdown handling for any page made of
//! common Markdown lines.

use mind_pkm::markdown::Document;
use mind_pkm::markdown::edit::{self, Sel};
use mind_pkm::markdown::live;
use mind_pkm::markdown::table::{self, Align, Table};
use proptest::prelude::*;

/// Lines that make up pages: every kind of block, nesting, and stray syntax.
const LINES: &[&str] = &[
    "",
    "",
    "text with **bold** and *em*",
    "more text",
    "# Heading",
    "## Sub *heading*",
    "Setext",
    "===",
    "- item",
    "  - nested",
    "    - deeper",
    "* [ ] task",
    "1. one",
    "2. two",
    "   continued",
    "> quote",
    "> > nested quote",
    ">",
    "```",
    "```rust",
    "fn main() {}",
    "$$",
    "x^2",
    "| a | b |",
    "|---|:-:|",
    "| 1 | 2 |",
    "[id]: http://example.com",
    "[^1]: a note",
    "    indented code",
    "---",
    "***",
    "<div>",
    "</div>",
    "[assets](My Page.assets)",
    "![img](a b.png)",
    "[toc]",
    "日本語のテキスト",
    "\ttab",
];

fn page() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(LINES), 0..25).prop_map(|lines| lines.join("\n"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn every_non_blank_byte_is_in_exactly_one_block(text in page()) {
        let doc = Document::parse(&text);
        let mut end = 0;
        for b in &doc.blocks {
            prop_assert!(b.range.start >= end, "overlapping blocks in {:?}: {:?}", text, doc.blocks);
            prop_assert!(b.range.start < b.range.end, "empty block in {:?}", text);
            let source = &text[b.range.clone()];
            prop_assert_eq!(source.trim_end(), source);
            prop_assert!(b.range.start == 0 || text.as_bytes()[b.range.start - 1] == b'\n',
                "block not at a line start in {:?}: {:?}", text, b);
            end = b.range.end;
        }
        for (i, c) in text.char_indices() {
            if !c.is_whitespace() {
                let n = doc.blocks.iter().filter(|b| b.range.contains(&i)).count();
                prop_assert_eq!(n, 1, "byte {} ({:?}) of {:?} is in {} blocks: {:?}", i, c, text, n, doc.blocks);
            }
        }
    }

    #[test]
    fn enter_inside_a_block_changes_only_that_block(text in page(), pick in any::<prop::sample::Index>(), at in any::<prop::sample::Index>(), shift in any::<bool>()) {
        let doc = Document::parse(&text);
        prop_assume!(!doc.blocks.is_empty());
        let block = &doc.blocks[pick.index(doc.blocks.len())];
        let region = block.range.clone();
        // A cursor position inside the block, on a character boundary.
        let positions: Vec<usize> = (region.start..=region.end).filter(|&i| text.is_char_boundary(i)).collect();
        let c = positions[at.index(positions.len())];
        let result = if shift {
            edit::shift_enter(&text, &doc, region.clone(), Sel::caret(c))
        } else {
            edit::enter(&text, &doc, region.clone(), Sel::caret(c))
        };
        if let Some(e) = result {
            prop_assert_eq!(&e.text[..region.start], &text[..region.start], "before the block of {:?}", text);
            let tail = text.len() - region.end;
            prop_assert_eq!(&e.text[e.text.len() - tail..], &text[region.end..], "after the block of {:?}", text);
            prop_assert!(e.sel.head <= e.text.len());
        }
    }

    #[test]
    fn tables_survive_writing_and_reading(
        cells in prop::collection::vec(prop::collection::vec("[a-zA-Z0-9 |日本語*_<>\\\\]{0,8}", 1..5), 2..5),
        aligns in prop::collection::vec(0u8..4, 5),
    ) {
        let columns = cells[0].len();
        let rows: Vec<Vec<String>> = cells
            .iter()
            .map(|r| {
                let mut r: Vec<String> = r.iter().map(|c| table::text_to_cell(c.trim())).collect();
                r.resize(columns, String::new());
                r
            })
            .collect();
        let align = aligns[..columns]
            .iter()
            .map(|a| [Align::None, Align::Left, Align::Center, Align::Right][*a as usize])
            .collect();
        let t = Table { align, rows };
        let written = t.to_markdown();
        let read = Table::parse(&written).unwrap();
        prop_assert_eq!(&read, &t, "{}", written);
        prop_assert_eq!(read.to_markdown(), written.clone());
        // And the page parser sees one table with those columns.
        let doc = Document::parse(&written);
        prop_assert_eq!(doc.blocks.len(), 1);
        prop_assert_eq!(&doc.blocks[0].kind, &mind_pkm::markdown::BlockKind::Table);
    }
}

/// Pieces of inline Markdown, stray or paired.
const PIECES: &[&str] = &[
    "a",
    "b",
    " ",
    "  ",
    "**",
    "*",
    "_",
    "`",
    "[",
    "](x.md)",
    "==",
    "~",
    "^",
    "\\",
    "\\*",
    "&amp;",
    ":smile:",
    "<u>",
    "</u>",
    "$",
    "![i](p.png)",
    "日",
    "本",
    "\n",
    "# ",
    "- ",
    "> ",
    "1. ",
    "- [ ] ",
];

fn inline_source() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(PIECES), 0..16).prop_map(|p| p.concat())
}

proptest! {
    /// Every shown position of the block being edited maps into the block and
    /// back to itself, wherever the cursor is.
    #[test]
    fn shown_positions_map_back(text in inline_source(), cursor in 0usize..200) {
        let doc = Document::parse(&text);
        let cursor = (0..=cursor.min(text.len())).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
        if let Some(region) = edit::region_at(&doc, cursor)
            && let Some(block) = doc.block_at(cursor)
            && live::editable(&doc, block)
        {
            let d = live::Display::new(&doc, doc.block_events(block), region.clone(), cursor);
            for (i, _) in d.text.char_indices().chain([(d.text.len(), ' ')]).filter(|(i, _)| d.is_stop(*i)) {
                let s = d.to_source(i);
                prop_assert!(region.start <= s && s <= region.end, "{text:?} shown {:?}: {i} -> {s}", d.text);
                prop_assert_eq!(d.to_shown(s), i, "{:?} {:?} at {}", text, d.text, i);
            }
            // Deleting any shown range removes source inside the block only.
            for (i, _) in d.text.char_indices() {
                for r in d.deletion(i..d.text.len()) {
                    prop_assert!(region.start <= r.start && r.end <= region.end, "{text:?}: {r:?}");
                }
            }
        }
    }
}
