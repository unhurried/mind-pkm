//! The blank lines of a list. CommonMark makes a list tight or loose as a
//! whole: one blank line between two of its items, or between two blocks of
//! an item, makes every item a paragraph with room around it. The editor
//! writes a list it edits in one of two forms - tight, without blank lines
//! between its items and before the blocks of its items, or loose, with one
//! blank line in each of these places - and loose only when tight would make
//! some of its blocks something else (a paragraph would join the text above
//! it, `---` would make that text a heading). The parser itself decides
//! that: the tight form is parsed and kept when it has the same blocks.

use super::edit::container_prefix;
use super::parse::{BlockKind, Document, List, ListItem};
use std::ops::Range;

/// A list's two forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Tight,
    Loose,
}

/// A text replaced at `at` (`removed` bytes by `inserted` ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Splice {
    at: usize,
    removed: usize,
    inserted: usize,
}

/// Where the offsets of a text go when its lists are rewritten: an offset
/// inside a rewritten separator goes to its start, the end of the line
/// before it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shift {
    /// The splices of each rewrite, in the order they were made, each in the
    /// offsets of the text it was made on, ascending.
    steps: Vec<Vec<Splice>>,
}

impl Shift {
    pub fn is_empty(&self) -> bool {
        self.steps.iter().all(Vec::is_empty)
    }

    pub fn map(&self, offset: usize) -> usize {
        self.steps.iter().fold(offset, |o, step| {
            let mut out = o;
            for s in step {
                if o >= s.at + s.removed {
                    out = out + s.inserted - s.removed;
                } else if o > s.at {
                    out -= o - s.at;
                }
            }
            out
        })
    }

    /// Where the offsets of `old` go in `new`, one contiguous change of it.
    pub fn of_change(old: &str, new: &str) -> Shift {
        let (start, end) = common(old, new);
        let splice = Splice { at: start, removed: old.len() - end - start, inserted: new.len() - end - start };
        Shift { steps: vec![vec![splice]] }
    }

    /// This shift followed by `then`.
    pub fn then(mut self, then: Shift) -> Shift {
        self.steps.extend(then.steps);
        self
    }
}

/// The lengths of the common start and the common end of two texts (not
/// overlapping in the shorter one).
fn common(a: &str, b: &str) -> (usize, usize) {
    let start = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    let end = a.bytes().rev().zip(b.bytes().rev()).take_while(|(x, y)| x == y).count();
    (start, end.min(a.len() - start).min(b.len() - start))
}

/// The part of `new` an edit of `old` changed: between their common start and end.
pub fn changed(old: &str, new: &str) -> Range<usize> {
    let (start, end) = common(old, new);
    start..new.len() - end
}

/// The separators of `list` in `doc`: the byte ranges from the line break
/// ending a block's last line to the start of the line of the block after
/// it, with whether that block is one of the list's items (rather than a
/// block inside an item), and whether the block before it is an empty
/// item. Separators holding anything but blank lines are left out.
fn separators(doc: &Document, list: &List) -> Vec<(Range<usize>, bool, bool)> {
    let text = doc.text.as_str();
    let mut out = Vec::new();
    for &at in &list.separators {
        let Some(next) = doc.blocks.iter().position(|b| b.range.start == at) else { continue };
        let Some(prev) = next.checked_sub(1).map(|p| &doc.blocks[p]) else { continue };
        let Some(nl) = text[prev.range.end..].find('\n').map(|i| prev.range.end + i) else { continue };
        let end = text[..at].rfind('\n').map_or(0, |i| i + 1);
        if !text[prev.range.end..nl].chars().all(|c| c == ' ' || c == '\t') || end <= nl {
            continue;
        }
        if !text[nl..end].chars().all(|c| matches!(c, '\n' | ' ' | '\t' | '>')) {
            continue;
        }
        let item = matches!(&doc.blocks[next].kind, BlockKind::ListItem(i) if i.depth == list.depth);
        let empty_item = matches!(prev.kind, BlockKind::ListItem(_))
            && crate::markdown::edit::line_marker(&text[prev.range.clone()])
                .is_some_and(|m| text[prev.range.start + m.end..prev.range.end].trim().is_empty());
        out.push((nl..end, item, empty_item));
    }
    out
}

/// `doc`'s text with the separators of `list` written in `form`, and the
/// splices that makes.
fn rewrite(doc: &Document, list: &List, form: Form) -> (String, Vec<Splice>) {
    let text = doc.text.as_str();
    let blank = container_prefix(&list.outer, true);
    let mut out = String::with_capacity(text.len() + 64);
    let mut splices = Vec::new();
    let mut pos = 0;
    for (range, item, after_empty_item) in separators(doc, list) {
        // An empty item cannot have a blank line before its first block.
        let with =
            if form == Form::Tight || (after_empty_item && !item) { "\n".to_string() } else { format!("\n{blank}\n") };
        if text[range.clone()] == with {
            continue;
        }
        out.push_str(&text[pos..range.start]);
        out.push_str(&with);
        splices.push(Splice { at: range.start, removed: range.len(), inserted: with.len() });
        pos = range.end;
    }
    out.push_str(&text[pos..]);
    (out, splices)
}

/// What must stay the same when a list's blank lines change: each block's
/// kind (a list item's looseness and offsets aside), the kinds of the
/// containers around it, and its lines.
fn same_blocks(a: &Document, b: &Document) -> bool {
    fn key(doc: &Document, i: usize) -> (BlockKind, Vec<crate::markdown::parse::ContainerKind>, Vec<&str>) {
        let block = &doc.blocks[i];
        let kind = match &block.kind {
            BlockKind::ListItem(item) => {
                BlockKind::ListItem(ListItem { loose: false, subtree_end: 0, list: 0, ..item.clone() })
            }
            kind => kind.clone(),
        };
        let lines: Vec<&str> = doc.text[block.range.clone()].lines().map(str::trim).collect();
        (kind, doc.outer_kinds(i), lines)
    }
    a.blocks.len() == b.blocks.len() && (0..a.blocks.len()).all(|i| key(a, i) == key(b, i))
}

/// The form `list` of `doc` is written in after an edit: tight when that
/// keeps every block, else loose when that does (else none: it stays as it
/// is), with the text and the splices.
fn tidy_list(doc: &Document, list: &List) -> Option<(String, Vec<Splice>, Document)> {
    for form in [Form::Tight, Form::Loose] {
        let (text, splices) = rewrite(doc, list, form);
        if splices.is_empty() {
            return None;
        }
        let parsed = Document::parse(&text);
        if same_blocks(doc, &parsed) {
            return Some((text, splices, parsed));
        }
    }
    None
}

/// `text` with every list that overlaps or touches `changed` (and the lists
/// around it) written in its form, and where its offsets go.
pub fn tidy(text: &str, changed: Range<usize>) -> (String, Shift) {
    let (doc, shift) = tidy_parsed(text, changed);
    (doc.text, shift)
}

/// `tidy`, with the parse of the text it gives (so that it need not be
/// parsed again).
pub fn tidy_parsed(text: &str, changed: Range<usize>) -> (Document, Shift) {
    tidy_from(Document::parse(text), changed)
}

/// `tidy_parsed` for the text `doc` is the parse of (parsed in part from the
/// parse of the text before an edit, say: `Document::reparse`).
pub fn tidy_from(mut doc: Document, changed: Range<usize>) -> (Document, Shift) {
    let targets: Vec<(usize, usize)> = doc
        .lists
        .iter()
        .filter(|l| l.range.start <= changed.end && changed.start <= l.range.end)
        .map(|l| (l.range.start, l.depth))
        .collect();
    let mut shift = Shift::default();
    for (start, depth) in targets {
        let start = shift.map(start);
        let Some(list) = doc.lists.iter().find(|l| l.range.start == start && l.depth == depth) else { continue };
        if let Some((_, splices, parsed)) = tidy_list(&doc, list) {
            shift.steps.push(splices);
            doc = parsed;
        }
    }
    (doc, shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `text` after an edit anywhere in it.
    fn tidied(text: &str) -> String {
        tidy(text, 0..text.len()).0
    }

    #[test]
    fn a_list_is_written_tight_when_nothing_needs_blank_lines() {
        assert_eq!(tidied("* a\n\n* b\n* c\n"), "* a\n* b\n* c\n");
        assert_eq!(tidied("1. a\n\n2. b\n"), "1. a\n2. b\n");
        assert_eq!(tidied("* a\n\n  * b\n\n* c\n"), "* a\n  * b\n* c\n");
        // Blocks that may follow an item's text right away.
        for block in ["```\n  x\n  ```", "| a | b |\n  | --- | --- |\n  | 1 | 2 |", "> q", "# h", "***", "* n"] {
            let loose = format!("* a\n\n  {block}\n\n* c\n");
            assert_eq!(tidied(&loose), format!("* a\n  {block}\n* c\n"), "{block}");
        }
        // A blank line inside a code block is the code's.
        assert_eq!(tidied("* a\n\n  ```\n  x\n\n  y\n  ```\n"), "* a\n  ```\n  x\n\n  y\n  ```\n");
    }

    #[test]
    fn a_list_is_written_loose_when_a_block_needs_a_blank_line() {
        assert_eq!(tidied("* a\n\n  p\n* b\n* c\n"), "* a\n\n  p\n\n* b\n\n* c\n");
        for block in ["p", "![i](x.png)", "$$\n  x\n  $$", "---"] {
            let mixed = format!("* a\n* b\n\n  {block}\n* c\n");
            assert_eq!(tidied(&mixed), format!("* a\n\n* b\n\n  {block}\n\n* c\n"), "{block}");
        }
        // Several blank lines become one.
        assert_eq!(tidied("* a\n\n\n  p\n\n\n* b\n"), "* a\n\n  p\n\n* b\n");
    }

    #[test]
    fn nested_lists_take_their_own_form() {
        assert_eq!(tidied("* a\n\n  * x\n\n  * y\n\n  p\n* b\n"), "* a\n\n  * x\n  * y\n\n  p\n\n* b\n");
        assert_eq!(tidied("* a\n  * x\n\n    p\n  * y\n* b\n"), "* a\n  * x\n\n    p\n\n  * y\n* b\n");
    }

    #[test]
    fn a_list_in_a_quote_keeps_its_markers() {
        assert_eq!(tidied("> * a\n>\n> * b\n> * c\n"), "> * a\n> * b\n> * c\n");
        assert_eq!(tidied("> * a\n>\n>   p\n> * b\n"), "> * a\n>\n>   p\n>\n> * b\n");
    }

    #[test]
    fn only_the_lists_at_the_change_are_tidied() {
        let text = "* a\n\n* b\n\npara\n\n* c\n\n* d\n";
        assert_eq!(tidy(text, 1..1).0, "* a\n* b\n\npara\n\n* c\n\n* d\n");
        assert_eq!(tidy(text, 13..13).0, text);
        assert_eq!(tidy(text, 20..21).0, "* a\n\n* b\n\npara\n\n* c\n* d\n");
    }

    #[test]
    fn an_empty_item_keeps_its_first_block_close() {
        assert_eq!(tidied("* a\n\n  p\n* \n  * n\n"), "* a\n\n  p\n\n* \n  * n\n");
    }

    #[test]
    fn separators_with_other_text_stay() {
        // Not a separator: the blocks there are no list's.
        assert_eq!(tidied("* a\n\n<!-- c -->\n\n* b\n"), "* a\n\n<!-- c -->\n\n* b\n");
    }

    #[test]
    fn offsets_follow_the_rewrite() {
        let text = "* a\n\n* b\n\n* c\n";
        let (new, shift) = tidy(text, 0..0);
        assert_eq!(new, "* a\n* b\n* c\n");
        assert_eq!(shift.map(0), 0);
        assert_eq!(shift.map(3), 3);
        // On the blank line: the end of the line before.
        assert_eq!(shift.map(4), 3);
        // The start of `* b`, its text, and the end.
        assert_eq!(shift.map(5), 4);
        assert_eq!(shift.map(7), 6);
        assert_eq!(shift.map(text.len()), new.len());
        assert!(tidy("x\n", 0..0).1.is_empty());
    }

    #[test]
    fn the_parse_of_the_tidied_text_comes_with_it() {
        for (text, changed) in [("* a\n\n* b\n\npara\n", 0..0), ("x\n", 0..0), ("> * a\n>\n> * b\n", 3..3)] {
            let (doc, _) = tidy_parsed(text, changed);
            let again = Document::parse(&doc.text);
            assert_eq!(doc.events, again.events, "{text:?}");
            assert_eq!(doc.blocks.len(), again.blocks.len());
        }
    }

    #[test]
    fn the_changed_part_of_a_text() {
        assert_eq!(changed("* a\n* c", "* a\n* b\n* c"), 6..10);
        assert_eq!(changed("abc", "ac"), 1..1);
        assert_eq!(changed("aa", "aaa"), 2..3);
        let shift = Shift::of_change("one\n\n\n\ntwo", "one\n\ntwo");
        assert_eq!(shift.map(9), 7);
        assert_eq!(shift.map(2), 2);
    }
}
