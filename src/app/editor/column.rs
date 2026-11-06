//! What the page column shows, in order: the page's blocks, with the active
//! region in place of the blocks it covers (a new paragraph before the block
//! after it). For each entry: what it is, the containers around it, the
//! source it covers, the gap above it and its key. The view lays this out.

use super::page::Page;
use crate::markdown::BlockKind;
use crate::markdown::parse::ContainerKind;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

/// What an entry of the column shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// A block of the page's parse, rendered.
    Block(usize),
    /// The active region.
    Active,
}

/// What an entry is, for its widgets to keep their state (its shaped text)
/// while entries come and go around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKey {
    /// A rendered block, by its source alone (no offset, which changes with
    /// every edit above it): the `n`th block (from 0) whose source has this
    /// hash.
    Block { source: u64, n: usize },
    /// The region being edited, by its place among the entries: its widgets
    /// (the text being edited, its focus) keep their state while it stays
    /// there, and get a new one when it moves to another block.
    Active(usize),
}

/// The room above an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gap {
    /// The first entry has none.
    None,
    /// Items of a tight list stay close together.
    Tight,
    /// A heading gets more room above it.
    Heading,
    Normal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub shown: Shown,
    /// Its kind: the block's, the active region's; a new paragraph is a paragraph.
    pub kind: BlockKind,
    /// The containers around it, outermost first, with their index in the
    /// page's parse when known (a new paragraph's are not in it yet).
    pub outer: Vec<(ContainerKind, Option<usize>)>,
    /// The source it covers (where a new paragraph goes).
    pub covers: Range<usize>,
    pub gap: Gap,
    /// The containers it shares with the entry above, which go on through the
    /// gap between them (a quote's bar).
    pub shared: Vec<ContainerKind>,
    pub key: EntryKey,
}

impl Entry {
    /// The kinds of the containers around it, outermost first.
    pub fn kinds(&self) -> Vec<ContainerKind> {
        self.outer.iter().map(|(k, _)| *k).collect()
    }
}

/// The entries of the column for `page`.
pub fn entries(page: &Page) -> Vec<Entry> {
    let doc = &page.doc();
    let active = page.active();
    let mut shown = Vec::with_capacity(doc.blocks.len() + 1);
    let mut active_block = None;
    // Whether the active region is among them yet.
    let mut placed = false;
    for (i, block) in doc.blocks.iter().enumerate() {
        if let Some(a) = active {
            let overlaps = block.range.end >= a.region.start && block.range.start <= a.region.end;
            if page.pending().is_none() && overlaps {
                if !placed {
                    shown.push(Shown::Active);
                    (placed, active_block) = (true, Some(i));
                }
                continue;
            }
            // A new paragraph, or a block typed empty that covers none of
            // the parse: before the block after it.
            let before = match page.pending() {
                Some(pd) => pd.at <= block.range.start,
                None => a.region.end < block.range.start,
            };
            if !placed && before {
                shown.push(Shown::Active);
                placed = true;
            }
        }
        shown.push(Shown::Block(i));
    }
    if active.is_some() && !placed {
        shown.push(Shown::Active);
    }
    let of_block = |i: usize| -> (BlockKind, Vec<(ContainerKind, Option<usize>)>) {
        let b = &doc.blocks[i];
        (b.kind.clone(), b.outer.iter().map(|&c| (doc.containers[c].kind, Some(c))).collect())
    };
    let mut entries: Vec<Entry> = Vec::new();
    // How many blocks with a source of each hash came before.
    let mut sources: HashMap<u64, usize> = HashMap::new();
    for s in shown {
        let (kind, outer) = match (s, &page.pending(), active_block) {
            (Shown::Block(i), _, _) => of_block(i),
            (Shown::Active, Some(pd), _) => (BlockKind::Paragraph, pd.outer.iter().map(|&k| (k, None)).collect()),
            (Shown::Active, None, Some(i)) => (active.map_or(BlockKind::Paragraph, |a| a.kind.clone()), of_block(i).1),
            (Shown::Active, None, None) => (BlockKind::Paragraph, Vec::new()),
        };
        let covers = match (s, &page.pending(), active) {
            (Shown::Block(i), _, _) => doc.blocks[i].range.clone(),
            (Shown::Active, Some(pd), _) => pd.at..pd.at,
            (Shown::Active, None, Some(a)) => a.region.clone(),
            (Shown::Active, None, None) => 0..0,
        };
        let previous = entries.last();
        let gap = gap(previous.map(|p| &p.kind), &kind);
        let shared = previous.map(|p| shared(&p.outer, &outer)).unwrap_or_default();
        let key = match s {
            Shown::Block(_) => {
                let mut hasher = DefaultHasher::new();
                page.text()[covers.clone()].hash(&mut hasher);
                let source = hasher.finish();
                let n = sources.entry(source).or_default();
                *n += 1;
                EntryKey::Block { source, n: *n - 1 }
            }
            Shown::Active => EntryKey::Active(entries.len()),
        };
        entries.push(Entry { shown: s, kind, outer, covers, gap, shared, key });
    }
    entries
}

/// The room above a block of `kind` after one of `previous`.
fn gap(previous: Option<&BlockKind>, kind: &BlockKind) -> Gap {
    match (previous, kind) {
        (None, _) => Gap::None,
        (Some(BlockKind::ListItem(a)), BlockKind::ListItem(b)) if !b.loose && a.list == b.list => Gap::Tight,
        (_, BlockKind::Heading(_)) => Gap::Heading,
        _ => Gap::Normal,
    }
}

/// The containers two neighbors are both in, outermost first: the same
/// kind, and the same container when both are known.
fn shared(above: &[(ContainerKind, Option<usize>)], below: &[(ContainerKind, Option<usize>)]) -> Vec<ContainerKind> {
    above
        .iter()
        .zip(below)
        .take_while(|(a, b)| a.0 == b.0 && (a.1.is_none() || b.1.is_none() || a.1 == b.1))
        .map(|(a, _)| a.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::edit::{self, Sel};
    use iced::widget::text_editor::{Action, Edit};
    use std::path::PathBuf;

    fn page(text: &str) -> Page {
        Page::new(PathBuf::from("p.md"), text.to_string(), false)
    }

    fn shown(page: &Page) -> Vec<Shown> {
        entries(page).iter().map(|e| e.shown).collect()
    }

    #[test]
    fn blocks_with_the_room_between_them() {
        let p = page("# Title\n\nPara\n\n- a\n- b\n\n1. c\n\n## Next\n");
        assert_eq!(shown(&p), (0..6).map(Shown::Block).collect::<Vec<_>>());
        let gaps: Vec<Gap> = entries(&p).iter().map(|e| e.gap).collect();
        assert_eq!(gaps, [Gap::None, Gap::Normal, Gap::Normal, Gap::Tight, Gap::Normal, Gap::Heading]);
    }

    #[test]
    fn the_active_region_takes_the_place_of_its_block() {
        let mut p = page("one\n\ntwo\n\nthree\n");
        p.place(Sel::caret(6));
        let entries = entries(&p);
        assert_eq!(
            entries.iter().map(|e| e.shown).collect::<Vec<_>>(),
            [Shown::Block(0), Shown::Active, Shown::Block(2)]
        );
        assert_eq!((entries[1].kind.clone(), entries[1].covers.clone()), (BlockKind::Paragraph, 5..8));
    }

    #[test]
    fn a_new_paragraph_goes_before_the_block_after_it() {
        let mut p = page("one\n\ntwo\n");
        let e = edit::enter(p.text(), p.doc(), 0..3, Sel::caret(3)).unwrap();
        p.apply(e);
        let at = p.pending().unwrap().at;
        let entries = entries(&p);
        assert_eq!(
            entries.iter().map(|e| e.shown).collect::<Vec<_>>(),
            [Shown::Block(0), Shown::Active, Shown::Block(1)]
        );
        assert_eq!((entries[1].kind.clone(), entries[1].covers.clone()), (BlockKind::Paragraph, at..at));
    }

    #[test]
    fn a_block_typed_empty_stays_in_its_place() {
        // A new paragraph typed into, then its text deleted.
        let mut p = page("one\n\ntwo\n");
        let e = edit::enter(p.text(), p.doc(), 0..3, Sel::caret(3)).unwrap();
        p.apply(e);
        p.perform(Action::Edit(Edit::Insert('a')));
        p.perform(Action::Edit(Edit::Backspace));
        assert_eq!(shown(&p), [Shown::Block(0), Shown::Active, Shown::Block(1)]);
        // A paragraph's text deleted.
        let mut p = page("a\n\nb\n\nc\n");
        p.place(Sel::caret(4));
        p.perform(Action::Edit(Edit::Backspace));
        assert_eq!(p.text(), "a\n\n\n\nc\n");
        assert_eq!(shown(&p), [Shown::Block(0), Shown::Active, Shown::Block(1)]);
    }

    /// The rendered blocks' sources with their keys.
    fn keys(page: &Page) -> Vec<(String, EntryKey)> {
        entries(page)
            .iter()
            .filter(|e| e.shown != Shown::Active)
            .map(|e| (page.text()[e.covers.clone()].to_string(), e.key))
            .collect()
    }

    #[test]
    fn blocks_keep_their_keys_while_blocks_come_and_go_before_them() {
        let mut p = page("one\n\ntwo\n\n- a\n- b\n\n> c\n");
        p.place(Sel::caret(3));
        let after = |p: &Page| keys(p).into_iter().filter(|(s, _)| s != "one").collect::<Vec<_>>();
        let before = after(&p);
        assert_eq!(before.len(), 4);
        // Typed into: the offsets after it change, the keys do not.
        p.perform(Action::Edit(Edit::Insert('x')));
        assert_eq!(p.text(), "onex\n\ntwo\n\n- a\n- b\n\n> c\n");
        let after = |p: &Page| keys(p).into_iter().filter(|(s, _)| !s.starts_with("one")).collect::<Vec<_>>();
        assert_eq!(after(&p), before);
        // A new paragraph, typed into.
        let e = edit::enter(p.text(), p.doc(), 0..4, Sel::caret(4)).unwrap();
        p.apply(e);
        p.perform(Action::Edit(Edit::Insert('y')));
        assert_eq!(p.text(), "onex\n\ny\n\ntwo\n\n- a\n- b\n\n> c\n");
        assert_eq!(after(&p), before);
        // Emptied, then joined with the block before it again.
        p.perform(Action::Edit(Edit::Backspace));
        assert_eq!(after(&p), before);
        let region = p.active().unwrap().region.clone();
        let e = edit::backspace_nested(p.text(), p.doc(), region, p.sel().head).unwrap();
        p.apply(e);
        assert_eq!(p.text(), "onex\n\ntwo\n\n- a\n- b\n\n> c\n");
        assert_eq!(after(&p), before);
    }

    #[test]
    fn keys_are_unique() {
        let mut p = page("same\n\nsame\n\nother\n\nsame\n");
        p.place(Sel::caret(0));
        let entries = entries(&p);
        let keys: Vec<EntryKey> = entries.iter().map(|e| e.key).collect();
        assert_eq!(keys.iter().collect::<std::collections::HashSet<_>>().len(), keys.len(), "{keys:?}");
        // The region being edited by its place.
        assert_eq!(keys[0], EntryKey::Active(0));
    }

    #[test]
    fn blocks_in_a_quote_share_it_through_the_gap() {
        let p = page("before\n\n> one\n>\n> two\n");
        let entries = entries(&p);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].kinds(), [ContainerKind::Quote]);
        assert!(entries[1].shared.is_empty(), "the paragraph before is outside the quote");
        assert_eq!(entries[2].shared, [ContainerKind::Quote]);
    }
}
