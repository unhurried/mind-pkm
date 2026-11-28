//! Editable Markdown lines left between the parsed blocks of a page.

use super::edit::{container_prefix, line_range, splice};
use super::parse::{ContainerKind, prefix_len};
use super::{Block, BlockKind, Document};
use std::collections::HashSet;
use std::ops::Range;

#[derive(Debug)]
pub(crate) struct BlankLine {
    /// The entire source line, including its terminator if present.
    pub range: Range<usize>,
    pub at: usize,
    /// Index of the next block, or the number of blocks at the end of the page.
    pub before: usize,
    pub outer: Vec<usize>,
}

fn line_after(text: &str, at: usize) -> usize {
    let end = line_range(text, at).end;
    (end + 1).min(text.len())
}

fn gap(doc: &Document, before: usize) -> Range<usize> {
    let first = before.checked_sub(1).map_or(0, |i| {
        let block = &doc.blocks[i];
        if matches!(block.kind, BlockKind::Code { fenced: true, .. })
            && let Some((_, source)) = block.events.start.checked_sub(1).and_then(|i| doc.events.get(i))
            && (source.end == doc.text.len() || doc.text[..source.end].ends_with('\n'))
        {
            return source.end;
        }
        line_after(&doc.text, block.range.end)
    });
    let last = doc.blocks.get(before).map_or(doc.text.len(), |b| line_range(&doc.text, b.range.start).start);
    first.min(last)..last
}

/// Recognize the prefixes actually present. An unmarked line can be inside a
/// list item, but a blank line without a quote marker belongs outside the quote.
fn owner(doc: &Document, block: &Block, start: usize, line: &str) -> Vec<usize> {
    let mut outer = Vec::new();
    let mut rest = line;
    for &i in &block.outer {
        let c = &doc.containers[i];
        let first = match c.kind {
            ContainerKind::Quote => c.range.start,
            ContainerKind::Item { .. } => line_after(&doc.text, c.range.start),
        };
        if start < first || start > c.range.end {
            break;
        }
        let prefix = prefix_len(rest, [c.kind]);
        if prefix == 0 && c.kind == ContainerKind::Quote {
            break;
        }
        rest = &rest[prefix..];
        outer.push(i);
    }
    outer
}

fn blank_rows(doc: &Document, before: usize, list_separator: bool) -> (Vec<BlankLine>, Option<Range<usize>>) {
    let bounds = gap(doc, before);
    let previous = before.checked_sub(1).map(|i| &doc.blocks[i]);
    let next = doc.blocks.get(before);
    let mut at = bounds.start;
    let mut rows = Vec::new();
    for raw in doc.text[bounds.clone()].split_inclusive('\n') {
        let start = at;
        at += raw.len();
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let outer = [previous, next]
            .into_iter()
            .flatten()
            .map(|block| owner(doc, block, start, line))
            .max_by_key(Vec::len)
            .unwrap_or_default();
        let prefix = prefix_len(line, outer.iter().map(|&i| doc.containers[i].kind));
        if line[prefix..].trim_matches([' ', '\t']).is_empty() {
            rows.push(BlankLine { range: start..at, at: start + prefix, before, outer });
        }
    }
    if let (Some(previous), Some(next)) = (previous, next)
        && let Some((separator, _)) = rows.iter().enumerate().min_by_key(|(_, row)| row.outer.len())
        && (list_separator || needs_separator(doc, previous, next, bounds))
    {
        let reserved = rows.remove(separator).range;
        return (rows, Some(reserved));
    }
    (rows, None)
}

pub(crate) fn rows(doc: &Document) -> Vec<BlankLine> {
    let list_separators: HashSet<_> = doc.lists.iter().flat_map(|list| list.separators.iter().copied()).collect();
    (0..=doc.blocks.len())
        .flat_map(|i| {
            blank_rows(doc, i, doc.blocks.get(i).is_some_and(|next| list_separators.contains(&next.range.start))).0
        })
        .collect()
}

fn needs_separator(doc: &Document, previous: &Block, next: &Block, gap: Range<usize>) -> bool {
    // Most pages consist of ordinary paragraphs. Joining two such paragraphs
    // in the same container always changes their boundary.
    if previous.kind == BlockKind::Paragraph && next.kind == BlockKind::Paragraph && previous.outer == next.outer {
        return true;
    }
    if next.kind == BlockKind::Paragraph
        && (matches!(previous.kind, BlockKind::ListItem(_))
            || previous.outer.iter().any(|&i| matches!(doc.containers[i].kind, ContainerKind::Item { .. })))
        && (previous.outer != next.outer || matches!(previous.kind, BlockKind::ListItem(_)))
    {
        return true;
    }

    // Try adjacency in the original parse context shared by the two blocks.
    // Compare source boundaries too, so a change in block ownership or consumed
    // content is detected even if the number and kind of blocks stay the same.
    let start = line_range(&doc.text, previous.range.start).start;
    let end = line_range(&doc.text, next.range.end).end;
    let shared = previous.outer.iter().zip(&next.outer).take_while(|(a, b)| a == b).count();
    let kinds = previous.outer[..shared].iter().map(|&i| doc.containers[i].kind);
    let left = without_prefixes(&doc.text[start..gap.start], kinds.clone());
    let right = without_prefixes(&doc.text[gap.end..end], kinds.clone());
    let between = without_prefixes(&doc.text[gap], kinds);
    boundary_changes(&left, &between, &right)
}

fn without_prefixes(text: &str, kinds: impl Iterator<Item = ContainerKind> + Clone) -> String {
    text.split_inclusive('\n').map(|line| &line[prefix_len(line, kinds.clone())..]).collect()
}

fn boundary_changes(left: &str, between: &str, right: &str) -> bool {
    let spaced = Document::parse(&format!("{left}{between}{right}"));
    let joined = Document::parse(&format!("{left}{right}"));
    let strip = |offset: usize| {
        if offset >= left.len() + between.len() { offset - between.len() } else { offset }
    };
    spaced.blocks.len() != joined.blocks.len()
        || spaced.containers.len() != joined.containers.len()
        || spaced.blocks.iter().zip(&joined.blocks).any(|(a, b)| {
            std::mem::discriminant(&a.kind) != std::mem::discriminant(&b.kind)
                || a.outer != b.outer
                || (strip(a.range.start)..strip(a.range.end)) != b.range
        })
}

impl BlankLine {
    pub fn kinds(&self, doc: &Document) -> Vec<ContainerKind> {
        self.outer.iter().map(|&i| doc.containers[i].kind).collect()
    }

    /// Insert the next row, completing an unterminated source line if needed.
    pub fn insert_after(&self, doc: &Document) -> (String, usize) {
        let prefix = container_prefix(&self.kinds(doc), true);
        let end = self.range.end;
        let terminate = if doc.text[..end].ends_with('\n') { "" } else { "\n" };
        let text = splice(&doc.text, end..end, &format!("{terminate}{prefix}\n"));
        (text, end + terminate.len() + prefix.len())
    }

    /// Write into this line and supply the separators for the new paragraph.
    pub fn write(&self, doc: &Document, text: &str) -> (String, usize, usize) {
        let kinds = self.kinds(doc);
        let blank = container_prefix(&kinds, true);
        let prefix = container_prefix(&kinds, false);
        let previous = self.before.checked_sub(1).map(|i| &doc.blocks[i]);
        let next = doc.blocks.get(self.before);
        let list_separator = is_list_separator(doc, self.before);
        let (_, reserved) = blank_rows(doc, self.before, list_separator);
        let is_missing = |above| reserved.as_ref().is_none_or(|r| (r.end <= self.range.start) != above);
        let needs = |block: &Block, above| {
            let depth = self.outer.iter().zip(&block.outer).take_while(|(a, b)| a == b).count();
            let source = without_prefixes(&doc.text[block.range.clone()], kinds[..depth].iter().copied());
            let typed = format!("{}text", container_prefix(&kinds[depth..], false));
            let separator = format!("{}\n", container_prefix(&kinds[depth..], true));
            if above {
                boundary_changes(&format!("{source}\n"), &separator, &typed)
            } else {
                boundary_changes(&format!("{typed}\n"), &separator, &source)
            }
        };
        let before = previous.is_some_and(|b| is_missing(true) && needs(b, true));
        let after = next.is_some_and(|b| is_missing(false) && (list_separator || needs(b, false)));
        let mut replacement = String::new();
        if before {
            replacement.push_str(&blank);
            replacement.push('\n');
        }
        replacement.push_str(&prefix);
        let start = self.range.start + replacement.len();
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                replacement.push('\n');
                replacement.push_str(if line.is_empty() { &blank } else { &prefix });
            }
            replacement.push_str(line);
        }
        let cursor = self.range.start + replacement.len();
        if doc.text[self.range.clone()].ends_with('\n') {
            replacement.push('\n');
        }
        if after {
            replacement.push_str(&blank);
            replacement.push('\n');
        }
        let page = splice(&doc.text, self.range.clone(), &replacement);
        (page, start, cursor)
    }
}

/// Add exactly one editable row beside a block. Creating a new gap can also
/// need a separator, for example before a paragraph in a tight list item.
pub(crate) fn insert_beside(doc: &Document, index: usize, before: bool) -> Option<(String, usize)> {
    let block = &doc.blocks[index];
    let gap_index = if before { index } else { index + 1 };
    let (old_rows, reserved) = blank_rows(doc, gap_index, is_list_separator(doc, gap_index));
    let line = if before {
        line_range(&doc.text, block.range.start).start
    } else {
        reserved.map_or_else(|| line_after(&doc.text, block.range.end), |line| line.end)
    };
    let prefix = container_prefix(&doc.outer_kinds(index), true);
    let separator = if !before && !doc.text[..line].ends_with('\n') { "\n" } else { "" };
    for count in 1..=2 {
        let insert = format!("{separator}{}", format!("{prefix}\n").repeat(count));
        let text = splice(&doc.text, line..line, &insert);
        let new = Document::parse(&text);
        if !super::lists::same_blocks(doc, &new) {
            continue;
        }
        let (new_rows, _) = blank_rows(&new, gap_index, is_list_separator(&new, gap_index));
        if new_rows.len() == old_rows.len() + 1
            && let Some(row) =
                new_rows.iter().rev().find(|r| r.range.start >= line && r.range.end <= line + insert.len())
        {
            return Some((text, row.at));
        }
    }
    None
}

fn is_list_separator(doc: &Document, before: usize) -> bool {
    doc.blocks.get(before).is_some_and(|next| doc.lists.iter().any(|list| list.separators.contains(&next.range.start)))
}

/// The old paragraph's line and any separators it no longer needs after
/// its text is deleted. Existing editable blank rows are kept.
pub(crate) fn vacated_lines(doc: &Document, index: usize, text: &str) -> Vec<Range<usize>> {
    let block = &doc.blocks[index];
    let start = line_range(&doc.text, block.range.start).start;
    let removed = doc.text.len() - text.len();
    let end = line_after(&doc.text, block.range.end) - removed;
    let without = splice(text, start..end, "");
    let new = Document::parse(&without);
    let (rows, _) = blank_rows(&new, index.min(new.blocks.len()), is_list_separator(&new, index));
    let (.., above) = blank_rows(doc, index, is_list_separator(doc, index));
    let (.., below) = blank_rows(doc, index + 1, is_list_separator(doc, index + 1));
    let mut cuts = Vec::with_capacity(3);
    if let Some(above) = above.filter(|r| rows.iter().any(|row| row.range == *r)) {
        cuts.push(above);
    }
    cuts.push(start..end);
    if let Some(below) = below.filter(|r| {
        let shift = removed + end - start;
        rows.iter().any(|row| row.range == (r.start - shift..r.end - shift))
    }) {
        cuts.push(below.start - removed..below.end - removed);
    }
    cuts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(source: &str) -> Vec<usize> {
        rows(&Document::parse(source)).into_iter().map(|line| line.at).collect()
    }

    #[test]
    fn rows_at_the_edges_and_in_gaps_before_any_block() {
        assert_eq!(at("\n\n"), [0, 1]);
        assert!(at("a\n").is_empty());
        assert_eq!(at("a\n\n \t"), [2, 3]);
        assert_eq!(at("\na\n\n\n\nb\n"), [0, 4, 5]);
        assert_eq!(at("a\n\n```\nx\n\n\n```\n\n"), [2, 15]);
        assert_eq!(at("a\n\n```\nx\n\n\n"), [2]);
        assert_eq!(at("a\n\n\n| h |\n| --- |\n| c |\n"), [2, 3]);
        assert_eq!(at("a\n\n\n---\n"), [3]);
    }

    #[test]
    fn blank_rows_follow_their_actual_container_prefixes() {
        let doc = Document::parse("intro\n\n> one\n>\n>\n> two\n>\n\n\nafter\n");
        let owners: Vec<_> = rows(&doc).iter().map(|line| line.kinds(&doc)).collect();
        assert_eq!(owners, [vec![], vec![ContainerKind::Quote], vec![ContainerKind::Quote], vec![]]);
        let doc = Document::parse("- a\n\n\n  next\n\n\n- b\n");
        let owners: Vec<_> = rows(&doc).iter().map(|line| line.kinds(&doc)).collect();
        assert_eq!(owners, [vec![ContainerKind::Item { depth: 0, indent: 2 }], vec![]]);
    }
}
