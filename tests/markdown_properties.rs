//! Property tests of the page editor's Markdown handling for any page made of
//! common Markdown lines.

use mind_pkm::markdown::edit::{self, Sel};
use mind_pkm::markdown::live;
use mind_pkm::markdown::parse::ContainerKind;
use mind_pkm::markdown::table::{self, Align, Table};
use mind_pkm::markdown::{BlockKind, Document};
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
    "> - quoted item",
    ">   - nested quoted item",
    "> # quoted heading",
    "> ```",
    "  > quote in an item",
    "  ```",
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

/// Enter (or Shift+Enter) at `c` in the block `region` of `text` leaves the
/// page before and after the block as it was. The one exception is specified:
/// Enter in a list item puts the new item among the blocks the item has below
/// its text (above its nested items, after its other blocks), and the blank
/// lines between them go where an empty item allows. There, what is below the
/// text stays line by line, in order, with at most the new item added.
fn enter_changes_only_the_block(
    text: &str,
    doc: &Document,
    region: std::ops::Range<usize>,
    c: usize,
    shift: bool,
) -> Result<(), TestCaseError> {
    let result = if shift {
        edit::shift_enter(text, doc, region.clone(), Sel::caret(c))
    } else {
        edit::enter(text, doc, region.clone(), Sel::caret(c))
    };
    if let Some(e) = result {
        prop_assert_eq!(&e.text[..region.start], &text[..region.start], "before the block of {:?}", text);
        let list_item = doc.blocks.iter().any(|b| b.range == region && matches!(b.kind, BlockKind::ListItem(_)));
        if list_item && !shift {
            let lines =
                |t: &str| t.lines().map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect::<Vec<_>>();
            let below = lines(&text[region.end..]);
            let got = lines(&e.text[region.start..]);
            let mut rest = got.iter();
            for line in &below {
                prop_assert!(rest.any(|l| l == line), "below the item {:?} lost {:?}: {:?}", text, line, e.text);
            }
            prop_assert!(got.len() <= lines(&text[region.start..]).len() + 1, "{:?} -> {:?}", text, e.text);
        } else {
            let tail = text.len() - region.end;
            prop_assert_eq!(&e.text[e.text.len() - tail..], &text[region.end..], "after the block of {:?}", text);
        }
        prop_assert!(e.sel.head <= e.text.len());
    }
    Ok(())
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
                // Lines of a quote's `>` alone between its blocks are in none
                // (inside a code block in the quote, in that one).
                let line = edit::line_range(&text, i);
                let quote_line = c == '>'
                    && text[line].chars().all(|c| c == '>' || c.is_whitespace())
                    && doc.containers.iter().any(|q| q.kind == ContainerKind::Quote && q.range.contains(&i));
                prop_assert!(n == 1 || (n == 0 && quote_line), "byte {} ({:?}) of {:?} is in {} blocks: {:?}", i, c, text, n, doc.blocks);
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
        enter_changes_only_the_block(&text, &doc, region, c, shift)?;
    }

    #[test]
    fn rules_inside_containers_stay_inside_them(text in page(), pick in any::<prop::sample::Index>(), at in any::<prop::sample::Index>(), rule in 0u8..6) {
        let doc = Document::parse(&text);
        prop_assume!(!doc.blocks.is_empty());
        let block = &doc.blocks[pick.index(doc.blocks.len())];
        let region = edit::region_at(&doc, block.range.start).unwrap_or(block.range.clone());
        let positions: Vec<usize> = (region.start..=region.end).filter(|&i| text.is_char_boundary(i)).collect();
        let c = positions[at.index(positions.len())];
        let sel = Sel::caret(c);
        let result = match rule {
            0 => edit::nested(&text, &doc, region.clone(), sel, edit::enter),
            1 => edit::nested(&text, &doc, region.clone(), sel, edit::shift_enter),
            2 => edit::backspace_nested(&text, &doc, region.clone(), c),
            3 => edit::nested(&text, &doc, region.clone(), sel, |t, d, r, s| edit::delete(t, d, r, s.head)),
            4 => edit::nested(&text, &doc, region.clone(), sel, |t, d, r, s| edit::indent(t, d, r, s, false)),
            _ => edit::nested(&text, &doc, region.clone(), sel, |t, d, r, s| edit::indent(t, d, r, s, true)),
        };
        if let Some(e) = result {
            for o in [e.sel.anchor, e.sel.head] {
                prop_assert!(o <= e.text.len() && e.text.is_char_boundary(o), "{:?} -> {:?}", text, e);
            }
            if let Some(p) = &e.pending {
                prop_assert!(p.at <= e.text.len() && e.text.is_char_boundary(p.at), "{:?} -> {:?}", text, e);
                let (typed, start) = p.materialize(&e.text, "x");
                prop_assert_eq!(&typed[start..start + 1], "x");
            }
            // Inside a container, the page before it stays.
            if let Some(&c) = block.outer.last() {
                let start = edit::line_range(&text, doc.containers[c].range.start).start;
                prop_assert_eq!(&e.text[..start], &text[..start], "before the container of {:?}", text);
            }
        }
    }

    #[test]
    fn an_undetermined_marker_is_a_byte_of_its_line(text in page(), at in any::<prop::sample::Index>(), seg in any::<prop::sample::Index>(), typed in "[ a#>*`-]{0,2}") {
        let doc = Document::parse(&text);
        let positions: Vec<usize> = (0..=text.len()).filter(|&i| text.is_char_boundary(i)).collect();
        let cursor = positions[at.index(positions.len())];
        let segment = positions[seg.index(positions.len())];
        let before = live::Shape::new(&mind_pkm::markdown::BlockKind::Paragraph, Vec::new());
        if let Some(m) = live::undetermined(&doc, cursor, segment, &before, &typed) {
            prop_assert!(m < cursor && text.as_bytes()[m].is_ascii() && text.as_bytes()[m] != b'\n');
            let masked = Document::parse_masked(&text, Some(m));
            prop_assert_eq!(&masked.text, &text);
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

#[test]
fn enter_at_the_end_of_an_item_with_a_nested_item_below_a_blank_line() {
    // Issue #59: found by `enter_inside_a_block_changes_only_that_block`.
    let text = "- item\n\n  - nested";
    let doc = Document::parse(text);
    let region = doc.blocks[0].range.clone();
    enter_changes_only_the_block(text, &doc, region.clone(), region.end, false).unwrap();
}
