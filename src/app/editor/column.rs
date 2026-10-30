//! What the page column shows, in order: the page's blocks, with the active
//! region in place of the blocks it covers (a new paragraph before the block
//! after it). For each entry: what it is, the containers around it, the
//! source it covers and the gap above it. The view lays this out.

use super::page::Page;
use crate::markdown::BlockKind;
use crate::markdown::parse::ContainerKind;
use std::ops::Range;

/// What an entry of the column shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// A block of the page's parse, rendered.
    Block(usize),
    /// The active region.
    Active,
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
}

impl Entry {
    /// The kinds of the containers around it, outermost first.
    pub fn kinds(&self) -> Vec<ContainerKind> {
        self.outer.iter().map(|(k, _)| *k).collect()
    }
}

/// The entries of the column for `page`.
pub fn entries(page: &Page) -> Vec<Entry> {
    let doc = &page.doc;
    let active = page.active.as_ref();
    let mut shown = Vec::new();
    let mut active_block = None;
    for (i, block) in doc.blocks.iter().enumerate() {
        if let Some(a) = active {
            let overlaps = block.range.end >= a.region.start && block.range.start <= a.region.end;
            let placed = shown.contains(&Shown::Active);
            if page.pending.is_none() && overlaps {
                if !placed {
                    shown.push(Shown::Active);
                    active_block = Some(i);
                }
                continue;
            }
            if !placed && page.pending.as_ref().is_some_and(|pd| pd.at <= block.range.start) {
                shown.push(Shown::Active);
            }
        }
        shown.push(Shown::Block(i));
    }
    if active.is_some() && !shown.contains(&Shown::Active) {
        shown.push(Shown::Active);
    }
    let of_block = |i: usize| -> (BlockKind, Vec<(ContainerKind, Option<usize>)>) {
        let b = &doc.blocks[i];
        (b.kind.clone(), b.outer.iter().map(|&c| (doc.containers[c].kind, Some(c))).collect())
    };
    let mut entries: Vec<Entry> = Vec::new();
    for s in shown {
        let (kind, outer) = match (s, &page.pending, active_block) {
            (Shown::Block(i), _, _) => of_block(i),
            (Shown::Active, Some(pd), _) => (BlockKind::Paragraph, pd.outer.iter().map(|&k| (k, None)).collect()),
            (Shown::Active, None, Some(i)) => (active.map_or(BlockKind::Paragraph, |a| a.kind.clone()), of_block(i).1),
            (Shown::Active, None, None) => (BlockKind::Paragraph, Vec::new()),
        };
        let covers = match (s, &page.pending, active) {
            (Shown::Block(i), _, _) => doc.blocks[i].range.clone(),
            (Shown::Active, Some(pd), _) => pd.at..pd.at,
            (Shown::Active, None, Some(a)) => a.region.clone(),
            (Shown::Active, None, None) => 0..0,
        };
        let previous = entries.last();
        let gap = gap(previous.map(|p| &p.kind), &kind);
        let shared = previous.map(|p| shared(&p.outer, &outer)).unwrap_or_default();
        entries.push(Entry { shown: s, kind, outer, covers, gap, shared });
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
        let e = edit::enter(&p.text.clone(), &p.doc.clone(), 0..3, Sel::caret(3)).unwrap();
        p.apply(e);
        let at = p.pending.as_ref().unwrap().at;
        let entries = entries(&p);
        assert_eq!(
            entries.iter().map(|e| e.shown).collect::<Vec<_>>(),
            [Shown::Block(0), Shown::Active, Shown::Block(1)]
        );
        assert_eq!((entries[1].kind.clone(), entries[1].covers.clone()), (BlockKind::Paragraph, at..at));
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
