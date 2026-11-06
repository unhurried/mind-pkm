//! Parsing a page again after an edit, only around where it changed.
//!
//! Where a top-level element starts after a blank line, nothing before it
//! carries over into it - unless the element before it is one a blank line
//! does not end (a list, an indented code block). So the page is parsed again
//! from the start of such an element at or before the change, to the end of
//! the first element after the change that follows a blank line the change
//! did not touch. If that element still starts there in the new parse,
//! everything before has ended and the page from there on parses as it did,
//! shifted; else the part grows to the next such element, and after a few
//! tries the whole page is parsed. So is a page with link reference or
//! footnote definitions, which apply to the whole page, with an empty
//! nested item's marker, which are items for the whole page or not at all,
//! or starting with `---` but no front matter (a closing line anywhere would
//! make it).
//!
//! In debug builds every result is checked against the whole parse.

use super::{
    Block, BlockKind, CodeBlockKind, Container, Document, Event, Events, List, Tag, headings, line_start,
    number_footnotes, usable_mask,
};

/// How many times the part parsed again grows to the next element before the
/// whole page is parsed instead.
const TRIES: usize = 3;

/// A top-level element of a parse.
struct Top {
    /// Its first event.
    event: usize,
    /// The start of its first line.
    line: usize,
    /// Only indentation precedes it on that line.
    clean: bool,
}

/// The top-level elements of the parse `events` of `text`.
fn top_level(events: &Events, text: &str) -> Vec<Top> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    for (i, (event, range)) in events.iter().enumerate() {
        if depth == 0 && !matches!(event, Event::End(_)) {
            let line = line_start(text, range.start);
            let clean = text[line..range.start].bytes().all(|c| c == b' ' || c == b'\t');
            out.push(Top { event: i, line, clean });
        }
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    out
}

/// Whether the line before the one starting at `line` is blank.
fn blank_before(text: &str, line: usize) -> bool {
    line > 0 && text[line_start(text, line - 1)..line - 1].bytes().all(|c| matches!(c, b' ' | b'\t' | b'\r'))
}

/// Whether an element started by `event` may go on after a blank line.
fn open_ended(event: &Event) -> bool {
    matches!(event, Event::Start(Tag::List(_) | Tag::CodeBlock(CodeBlockKind::Indented) | Tag::FootnoteDefinition(_)))
}

/// Whether the line starting at `line` would start front matter at the start
/// of a page (or of a part of it parsed on its own).
fn metadata_like(text: &str, line: usize) -> bool {
    text[line..].starts_with("---") || text[line..].starts_with("+++")
}

/// How long the common start of `a` and `b` is.
fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    const CHUNK: usize = 64;
    let n = a.len().min(b.len());
    let mut i = 0;
    while i + CHUNK <= n && a[i..i + CHUNK] == b[i..i + CHUNK] {
        i += CHUNK;
    }
    while i < n && a[i] == b[i] {
        i += 1;
    }
    i
}

/// How long the common end of `a` and `b` is.
fn common_suffix(a: &[u8], b: &[u8]) -> usize {
    const CHUNK: usize = 64;
    let n = a.len().min(b.len());
    let (mut i, (la, lb)) = (0, (a.len(), b.len()));
    while i + CHUNK <= n && a[la - i - CHUNK..la - i] == b[lb - i - CHUNK..lb - i] {
        i += CHUNK;
    }
    while i < n && a[la - i - 1] == b[lb - i - 1] {
        i += 1;
    }
    i
}

/// Where `old` (parsed with `old_mask`) and `new` (to be parsed with
/// `new_mask`) differ, in `old`: before the first and from the second on,
/// they are the same - the bytes parsed as letters counting as different.
fn changed(old: &str, new: &str, old_mask: Option<usize>, new_mask: Option<usize>) -> (usize, usize) {
    let (o, n) = (old.as_bytes(), new.as_bytes());
    let prefix = common_prefix(o, n);
    let suffix = common_suffix(&o[prefix..], &n[prefix..]);
    let (mut a, mut b) = (prefix, o.len() - suffix);
    let delta = n.len() as isize - o.len() as isize;
    if let Some(m) = old_mask {
        (a, b) = (a.min(m), b.max(m + 1));
    }
    if let Some(m) = new_mask {
        if m < a {
            a = m;
        } else if m as isize >= b as isize + delta {
            b = b.max((m as isize - delta) as usize + 1);
        }
    }
    (a, b)
}

impl Document {
    /// The parse of `text` with the byte at `mask` parsed as a letter, as
    /// `parse_masked` gives it - made from this parse of another version of
    /// the text, parsing again only around where they differ when that gives
    /// the same.
    pub fn reparse(self, text: &str, mask: Option<usize>) -> Document {
        let mask = usable_mask(text, mask);
        let parsed = self.reparse_in_part(text, mask).unwrap_or_else(|| Document::parse_masked(text, mask));
        #[cfg(debug_assertions)]
        assert!(
            parsed == Document::parse_masked(text, mask),
            "parsed again in part, a page differs from its whole parse (mask {mask:?}):\n{text}"
        );
        parsed
    }

    fn reparse_in_part(self, text: &str, mask: Option<usize>) -> Option<Document> {
        if self.whole {
            return None;
        }
        if self.text == text && self.mask == mask {
            return Some(self);
        }
        // A page starting with `---` has front matter when a closing line
        // follows, anywhere: an edit anywhere may add one - unless the page
        // has it already (its closing line is the first one).
        let front = matches!(self.events.first(), Some((Event::Start(Tag::MetadataBlock(_)), _)));
        if metadata_like(text, 0) && !front {
            return None;
        }
        let old = self.text.as_str();
        let (a, b) = changed(old, text, self.mask, mask);
        let delta = text.len() as isize - old.len() as isize;
        let shift = |at: usize| (at as isize + delta) as usize;
        let tops = top_level(&self.events, old);
        if tops.is_empty() {
            return None;
        }
        // The part starts with the element the change starts in (or the last
        // one before it), or before, where the page parses on its own.
        // (Its line is where it was, before the change; what it holds may not
        // be: the part parsed starts with the new text there.)
        let alone = |k: usize| {
            tops[k].clean
                && blank_before(old, tops[k].line)
                && !open_ended(&self.events[tops[k - 1].event].0)
                && !metadata_like(text, tops[k].line)
        };
        let mut k = tops.partition_point(|t| t.line <= a).saturating_sub(1);
        while k > 0 && !alone(k) {
            k -= 1;
        }
        let (start, first_event) = if k == 0 { (0, 0) } else { (tops[k].line, tops[k].event) };
        // It goes to the end of the first element after the change that
        // follows a blank line the change did not touch.
        let follows_blank = |j: usize| {
            let t = &tops[j];
            t.clean && blank_before(old, t.line) && line_start(old, t.line - 1) >= b
        };
        let mut j = tops.partition_point(|t| t.line < b);
        for _ in 0..TRIES {
            while j < tops.len() && !follows_blank(j) {
                j += 1;
            }
            let sentinel = (j < tops.len()).then(|| (tops[j].event, tops[j].line));
            let end = match sentinel {
                Some(_) => shift(tops.get(j + 1).map_or(old.len(), |t| t.line)),
                None => text.len(),
            };
            if mask.is_some_and(|m| !(start..end).contains(&m)) {
                return None;
            }
            let part = Document::parse_masked(&text[start..end], mask.map(|m| m - start));
            if part.whole {
                return None;
            }
            // Front matter at the start of the page that does not end in the
            // part may end further down.
            if start == 0
                && metadata_like(text, 0)
                && !matches!(part.events.first(), Some((Event::Start(Tag::MetadataBlock(_)), _)))
            {
                return None;
            }
            // Where the element after the change starts in the new parse,
            // everything before it ended.
            let until = match sentinel {
                None => Some(part.events.len()),
                Some((_, line)) => {
                    let at = shift(line) - start;
                    top_level(&part.events, &part.text).iter().find(|t| t.line == at && t.clean).map(|t| t.event)
                }
            };
            match (until, sentinel) {
                (Some(until), _) => return Some(self.splice(text, mask, start, first_event, part, until, sentinel)),
                (None, Some(_)) => j += 1,
                (None, None) => return None,
            }
        }
        None
    }

    /// This parse with the part from `start` (its first event `first_event`)
    /// to the element `sentinel` (its first event and line; None: the end)
    /// replaced by `part`'s events before `until` (and the blocks, containers
    /// and lists that start there), for `text`.
    #[allow(clippy::too_many_arguments)]
    fn splice(
        mut self,
        text: &str,
        mask: Option<usize>,
        start: usize,
        first_event: usize,
        part: Document,
        until: usize,
        sentinel: Option<(usize, usize)>,
    ) -> Document {
        let delta = text.len() as isize - self.text.len() as isize;
        let shift = |at: usize| (at as isize + delta) as usize;
        let (end_event, end) = sentinel.unwrap_or((self.events.len(), self.text.len()));
        // Where the part ends in its own text.
        let part_end = sentinel.map_or(part.text.len(), |(_, line)| shift(line) - start);
        let Document {
            events: part_events, blocks: part_blocks, containers: part_containers, lists: part_lists, ..
        } = part;

        // Events: the part's in place of the old ones, those after shifted.
        let new_events = part_events.into_iter().take(until).map(|(e, r)| (e, r.start + start..r.end + start));
        self.events.splice(first_event..end_event, new_events);
        let after = first_event + until;
        for (_, r) in &mut self.events[after..] {
            *r = shift(r.start)..shift(r.end);
        }
        let event_shift = after as isize - end_event as isize;

        // Containers, which blocks refer to by index.
        let c_from = self.containers.partition_point(|c| c.range.start < start);
        let c_to = self.containers.partition_point(|c| c.range.start < end);
        let c_part = part_containers.partition_point(|c| c.range.start < part_end);
        let containers = part_containers
            .into_iter()
            .take(c_part)
            .map(|c| Container { range: c.range.start + start..c.range.end + start, ..c });
        self.containers.splice(c_from..c_to, containers);
        for c in &mut self.containers[c_from + c_part..] {
            c.range = shift(c.range.start)..shift(c.range.end);
        }
        let container_shift = (c_from + c_part) as isize - c_to as isize;

        // Blocks.
        let moved = |b: &mut Block, by: &dyn Fn(usize) -> usize, events: isize, outer: isize| {
            b.range = by(b.range.start)..by(b.range.end);
            // A block no element covers has no events.
            if b.kind != BlockKind::Other {
                let ev = |i: usize| (i as isize + events) as usize;
                b.events = ev(b.events.start)..ev(b.events.end);
            }
            for c in &mut b.outer {
                *c = (*c as isize + outer) as usize;
            }
            if let BlockKind::ListItem(item) = &mut b.kind {
                (item.subtree_end, item.list) = (by(item.subtree_end), by(item.list));
            }
        };
        let b_from = self.blocks.partition_point(|b| b.range.start < start);
        let b_to = self.blocks.partition_point(|b| b.range.start < end);
        let b_part = part_blocks.partition_point(|b| b.range.start < part_end);
        let into_page = |at: usize| at + start;
        let blocks = part_blocks.into_iter().take(b_part).map(|mut b| {
            moved(&mut b, &into_page, first_event as isize, c_from as isize);
            b
        });
        self.blocks.splice(b_from..b_to, blocks);
        for b in &mut self.blocks[b_from + b_part..] {
            moved(b, &shift, event_shift, container_shift);
        }

        // Lists.
        let l_from = self.lists.partition_point(|l| l.range.start < start);
        let l_to = self.lists.partition_point(|l| l.range.start < end);
        let l_part = part_lists.partition_point(|l| l.range.start < part_end);
        let lists = part_lists.into_iter().take(l_part).map(|l| List {
            range: l.range.start + start..l.range.end + start,
            separators: l.separators.iter().map(|&s| s + start).collect(),
            ..l
        });
        self.lists.splice(l_from..l_to, lists);
        for l in &mut self.lists[l_from + l_part..] {
            l.range = shift(l.range.start)..shift(l.range.end);
            for s in &mut l.separators {
                *s = shift(*s);
            }
        }

        self.text = text.to_string();
        self.mask = mask;
        self.footnotes = number_footnotes(&self.events);
        self.headings = headings(&self);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `text` parsed, then parsed again as `edited` (`reparse` checks it
    /// against the whole parse in debug builds); whether that was in part.
    fn in_part(text: &str, edited: &str, mask: Option<usize>) -> bool {
        let doc = Document::parse(text);
        let whole = Document::parse_masked(edited, mask);
        let part = doc.clone().reparse_in_part(edited, usable_mask(edited, mask));
        if let Some(p) = &part {
            assert_eq!(p, &whole, "{text:?} -> {edited:?}");
        }
        assert_eq!(doc.reparse(edited, mask), whole);
        part.is_some()
    }

    fn page(sections: usize) -> String {
        (0..sections)
            .map(|i| format!("## Section {i}\n\nText {i} with **bold**.\n\n- a {i}\n- b\n\n```\ncode {i}\n```\n\n| a | b |\n| - | - |\n| {i} | x |\n"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn an_edit_in_a_paragraph_is_parsed_in_part() {
        let text = page(20);
        let at = text.find("Text 7 with").unwrap() + 4;
        let edited = format!("{}x{}", &text[..at], &text[at..]);
        assert!(in_part(&text, &edited, None));
        // A marker being typed: parsed as a letter.
        let star = format!("{}*{}", &text[..at], &text[at..]);
        assert!(in_part(&text, &star, Some(at)));
        // The same text with the mask gone.
        assert!(in_part(&star, &star, None));
    }

    #[test]
    fn edits_that_reach_past_the_part_are_parsed_as_they_should() {
        let text = page(6);
        let at = text.find("Text 2 with").unwrap();
        // An unclosed fence takes the rest of the page.
        let fence = format!("{}```\n{}", &text[..at], &text[at..]);
        in_part(&text, &fence, None);
        // A paragraph turned into a list item joins the list after it.
        let item = format!("{}- {}", &text[..at], &text[at..]);
        in_part(&text, &item, None);
        // Indented text after a list continues it.
        let after_list = text.find("```\ncode 2").unwrap();
        let indented = format!("{}  x\n\n{}", &text[..after_list], &text[after_list..]);
        in_part(&text, &indented, None);
        // A blank line removed, joining two elements.
        let gap = text.find("\n\n- a 3").unwrap();
        let joined = format!("{}{}", &text[..gap], &text[gap + 1..]);
        in_part(&text, &joined, None);
        // An HTML comment opened and left open.
        let comment = format!("{}<!--\n{}", &text[..at], &text[at..]);
        in_part(&text, &comment, None);
        // Everything gone, or all new.
        in_part(&text, "", None);
        in_part("", &text, None);
    }

    #[test]
    fn definitions_make_the_whole_page_parse_again() {
        let text = format!("{}\n[d]: http://example.com\n", page(3));
        let at = text.find("Text 1").unwrap();
        let edited = format!("{}[x][d] {}", &text[..at], &text[at..]);
        assert!(!in_part(&text, &edited, None));
        let footnote = format!("{}[^1] {}", &page(3)[..at], &page(3)[at..]);
        assert!(!in_part(&page(3), &footnote, None));
    }

    #[test]
    fn front_matter_is_parsed_at_the_start_only() {
        let text = format!("---\ntitle: x\n---\n\n{}", page(3));
        let at = text.find("Text 1").unwrap();
        let edited = format!("{}y{}", &text[..at], &text[at..]);
        assert!(in_part(&text, &edited, None));
        // A rule that would start a metadata block at the start of a part.
        let rule = page(3).replace("\n\n- a 1", "\n\n---\n\nx: y\n\n---\n\n- a 1");
        let at = rule.find("x: y").unwrap();
        in_part(&rule, &format!("{}z{}", &rule[..at], &rule[at..]), None);
    }

    /// Lines pages are made of: every kind of block, nesting, stray syntax.
    const LINES: &[&str] = &[
        "",
        "",
        "",
        "text with **bold** and *em*",
        "more text",
        "# Heading",
        "Setext",
        "===",
        "- item",
        "  - nested",
        "* [ ] task",
        "1. one",
        "2. two",
        "   continued",
        "> quote",
        "> > nested quote",
        ">",
        "> - quoted item",
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
        "    indented code",
        "---",
        "***",
        "<div>",
        "</div>",
        "<!-- note",
        "-->",
        "[assets](My Page.assets)",
        "![img](a b.png)",
        "[toc]",
        "日本語のテキスト",
        "\ttab",
        "+ plus item",
        "1) paren item",
        "- [x] done",
        "    - deep item",
        "> > > deep quote",
        "~~~",
        "<pre>",
        "</pre>",
        "<!-- c -->",
        "* * *",
        "  continued in an item",
        "crlf line\r",
        "- ",
        ">   ```",
    ];

    /// What an edit puts in.
    const INSERTS: &[&str] = &[
        "",
        "x",
        " ",
        "\n",
        "\n\n",
        "- ",
        "  ",
        "    ",
        "> ",
        "# ",
        "```",
        "```\n",
        "$$",
        "|",
        "| a |\n|---|\n",
        "---",
        "***",
        "<div>",
        "</div>",
        "<!--",
        "-->",
        "1. ",
        "2. ",
        "* [ ] ",
        "===",
        "\t",
        "*",
        "**",
        "`",
        "[",
        "](a b)",
        "日本",
        "[^1]",
        "[d]: x",
        "~~~",
        "<pre>",
        "</pre>",
        "+ ",
        "1) ",
        "\n   ",
        ">\n",
        "    - ",
        "\r\n",
        "---\n",
        "\n---\nx: y\n---\n",
    ];

    use proptest::prelude::*;
    use std::cell::Cell;

    thread_local! {
        static IN_PART: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(3000))]

        /// Any edit of any page parses again as the whole page does.
        #[test]
        fn a_page_parsed_again_is_parsed_as_a_whole(
            lines in prop::collection::vec(prop::sample::select(LINES), 0..40),
            at in 0.0..=1.0f64,
            len in 0usize..12,
            insert in prop::sample::select(INSERTS),
            masked in any::<bool>(),
        ) {
            let text = lines.join("\n");
            let snap = |mut i: usize| { while !text.is_char_boundary(i) { i -= 1 } i };
            let from = snap((text.len() as f64 * at) as usize);
            let to = snap((from + len).min(text.len()));
            let edited = format!("{}{}{}", &text[..from], insert, &text[to..]);
            // A marker being typed (one character): parsed as a letter.
            let mask = (masked && insert.len() == 1).then_some(from);
            let doc = Document::parse(&text);
            let whole = Document::parse_masked(&edited, mask);
            let part = doc.clone().reparse_in_part(&edited, usable_mask(&edited, mask));
            IN_PART.with(|c| {
                let (n, of) = c.get();
                c.set((n + usize::from(part.is_some()), of + 1));
            });
            if let Some(part) = part {
                prop_assert_eq!(part, whole.clone(), "{:?} -> {:?}", text, edited);
            }
            prop_assert_eq!(doc.reparse(&edited, mask), whole);
        }
    }

    #[test]
    fn most_edits_are_parsed_in_part() {
        a_page_parsed_again_is_parsed_as_a_whole();
        let (n, of) = IN_PART.with(Cell::get);
        assert!(n * 2 > of, "{n} of {of} in part");
    }
}
