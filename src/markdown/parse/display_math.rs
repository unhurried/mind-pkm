//! Display math spanning a blank line, which divides CommonMark paragraphs.

use super::{BlockKind, Document, Event, Tag, TagEnd, prefix_len};
use crate::markdown::edit::line_range;

pub(super) fn has_delimiter(text: &str) -> bool {
    text.match_indices("$$").any(|(at, _)| {
        let line = line_range(text, at);
        text[line.start..at].bytes().all(|c| matches!(c, b' ' | b'\t' | b'>'))
            && text[at + 2..line.end].trim_matches([' ', '\t', '\r']).is_empty()
    })
}

/// Joins the sibling blocks within standalone display delimiters, keeping the
/// source ranges and the original TeX (including blank lines). Current inline
/// and display math already recognized by pulldown-cmark stays in its events.
pub(super) fn restore(doc: &mut Document) -> bool {
    let text = &doc.text;
    let mut replacements = Vec::new();
    let mut index = 0;
    while let Some(first) = doc.blocks.get(index) {
        index += 1;
        if first.kind != BlockKind::Paragraph {
            continue;
        }
        let first_line = line_range(text, first.range.start);
        let kinds = first.outer.iter().map(|&i| doc.containers[i].kind);
        let prefix = |line: &str| prefix_len(line, kinds.clone());
        let delimiter = |line: &str| line[prefix(line)..].trim_matches([' ', '\t', '\r']) == "$$";
        let line = &text[first_line.clone()];
        let end_line = line_range(text, first.range.end);
        if !delimiter(line) || (end_line.start > first_line.start && delimiter(&text[end_line])) {
            continue;
        }
        // The closing delimiter must end a paragraph in the same quote or item.
        let Some((end, last)) = doc
            .blocks
            .iter()
            .enumerate()
            .skip(index)
            .take_while(|(_, b)| {
                b.outer == first.outer && matches!(b.kind, BlockKind::Paragraph | BlockKind::Code { fenced: false, .. })
            })
            .find(|(_, b)| b.kind == BlockKind::Paragraph && delimiter(&text[line_range(text, b.range.end)]))
        else {
            continue;
        };
        let last_line = line_range(text, last.range.end);
        let Some(body) = text.get(first_line.end + 1..last_line.start) else { continue };
        if !body.lines().any(|line| line[prefix(line)..].trim_matches([' ', '\t']).is_empty()) {
            continue;
        }
        let Some(first_event) = first.events.start.checked_sub(1) else { continue };
        let last_event = last.events.end;
        let Some(owned) = doc.events.get(first_event..=last_event) else { continue };
        if !matches!(owned.first(), Some((Event::Start(Tag::Paragraph), _)))
            || !matches!(owned.last(), Some((Event::End(TagEnd::Paragraph), _)))
        {
            continue;
        }
        let opening = first_line.start + line.find("$$").unwrap();
        let closing = last_line.start + text[last_line].find("$$").unwrap() + 2;
        let wrapper = doc.events[first_event].1.start..doc.events[last_event].1.end;
        let mut tex = String::from("\n");
        for line in body.lines() {
            tex.push_str(&line[prefix(line)..]);
            tex.push('\n');
        }
        replacements.push((
            first_event..=last_event,
            [
                (Event::Start(Tag::Paragraph), wrapper.clone()),
                (Event::DisplayMath(tex.into()), opening..closing),
                (Event::End(TagEnd::Paragraph), wrapper),
            ],
        ));
        index = end + 1;
    }
    let changed = !replacements.is_empty();
    for (range, events) in replacements.into_iter().rev() {
        doc.events.splice(range, events);
    }
    changed
}
