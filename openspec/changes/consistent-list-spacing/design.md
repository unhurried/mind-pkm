# Design

## Context

`Document::parse` (`src/markdown/parse.rs`) turns a list into one `ListItem` block per item plus a block for each further block of an item (`Block::outer` points at the item's `ContainerKind::Item` container, created only when an item has such blocks). `ListItem::loose` is pulldown-cmark's looseness of the item's list, and the page column (`app/editor/column.rs`, `gap`) shows the items of a loose list with normal room between them and those of a tight list close together. So the display already follows the parser; only the source is inconsistent.

The blank lines come from the editing rules in `src/markdown/edit.rs`: `enter_list_item` separates a new item with `\n\n` when the list is loose, `into_previous_item` always puts a blank line before the paragraph it makes, and `Pending::after` / `Pending::gap` / `gap_pending` put blank lines on both sides of a new empty paragraph or an inserted block. `Editor::insert_block` (`app/editor/mod.rs`) places a new block with `gap_pending` at the end of the active block; for a list item's text that position is outside the item's container (the container starts on the item's second line), so the block lands at the top level and splits the list.

The page editor never regenerates text: edits splice the source, and the page is re-parsed after every change. Offsets held by the page (selection, active region, pending paragraph) must follow any splice.

## Goals / Non-Goals

**Goals:**
- One pure function writes a list tight or loose, deciding by the parser itself whether tight is possible.
- It runs only for the lists an editing command or a typed-in block changed, at moments where offsets can be mapped.
- Blocks inserted while a list item is active go into the item.

**Non-Goals:**
- A command to choose a list's form (tight / loose) by hand.
- Tidying lists on open or save, or in source mode, find and replace, renaming asset links, or undo / redo.
- Any change to rendering, HTML / PDF export, or how lists are parsed for display.

## Decisions

- **The lists of a page.** `Document` gets `lists: Vec<List>`, filled by `parse.rs`'s `list()`: for each list (nested ones too) its range, its depth, the containers around it, and its *separators*, each given by the index of the block that follows it: every item's block after the first item's, and the first block of each further block of an item - a nested list (its first item), a quote (its first block), any other block, or the text of a tight item after a nested list. The separator's text runs from the line break ending the previous block's last line to the start of the line of the block after it. Nested lists own their own separators; a list's separators never overlap another's.
- **The two forms.** Tight: the separator is one line break. Loose: a line break, one blank line, a line break. The blank line is the first blank line already in the separator when there is one (it carries the right `>`s), else the blank prefix of the containers around the list (`edit::container_prefix(kinds, true)`, `>` inside a quote). A separator holding anything but line breaks, spaces, tabs and `>` (an HTML comment, say) is left alone.
- **Tight when the parser agrees.** For a list, the tight candidate (its separators made tight) is parsed and compared with the page as it is, block by block: the same number of blocks, the same kinds (a list item's looseness and offsets aside), the same container kinds around each, and the same text line by line (trimmed). Equal: the list is written tight. Otherwise loose, after the same check on the loose candidate; if that fails too, the list is left as it is. Alternative considered: a table of block kinds that may follow an item's text without a blank line (fenced code, quote, ATX heading, table, `***`). Rejected: whether a block survives without a blank line depends on what precedes it (a line after a table or a quote continues it lazily; a nested ordered list must start at 1; `---` after text makes a setext heading; HTML blocks of kind 7 cannot interrupt a paragraph) and on pulldown-cmark's extensions; asking the parser keeps it right by construction. The form is a pure function of the text, so it does not matter which edit or keystroke last ran it.
- **Which lists.** The changed range is the part of the new text between the old and new texts' common prefix and suffix. Every list whose range overlaps or touches it (so the lists around it, and lists merged or split by the edit) is tidied, one at a time, re-parsing in between and finding the next list by its start offset carried through the previous splices. Lists elsewhere on the page are not touched.
- **The function.** `markdown/lists.rs` (new, UI-free): `tidy(text, changed: Range<usize>) -> (String, Shift)`, where `Shift` maps offsets of the old text into the new one (an offset inside a rewritten separator goes to its start, where a new paragraph between blocks is placed). Unit tests there pin the forms and the parser check.
- **When.** In `Page`:
  - `apply` (the outcome of every editing rule: Enter, Backspace, Delete, Tab, formatting, pasting text) tidies before re-parsing and maps the edit's selection through the `Shift`, in the same undo step. An outcome with a pending (empty, not yet typed) paragraph is not tidied then: its paragraph is not in the text yet and sits in a separator; the page remembers that it has an untidy edit instead.
  - Typing in a block (rendered text, a code block, a code-like block's source) only remembers the untidy edit; nothing moves while typing.
  - `deactivate` (leaving the active block or an untyped pending paragraph) tidies around the block left when an untidy edit is remembered, after `collapse_blank_lines`, like that cleanup without an undo step of its own (Undo of the typing goes back to the text before it). It returns the `Shift`, and `place` maps the position it was given through it before choosing the next block; this also fixes the offsets `collapse_blank_lines` moves today. Callers outside `Page` pass block indices or positions read afterwards, and tidying never changes the number or order of blocks (the parser check guarantees it).
  - The block commands that replace the text wholesale but are editing commands - inserting a block, deleting selected blocks, pasting over selected blocks - go through `apply` instead of `set_text`. `set_text` stays a plain replacement for find and replace, asset renames, source mode and the code block's language.
  Alternative considered: tidying after every keystroke. Rejected: lines would come and go under the cursor while a marker is typed (`` ` `` makes a paragraph, so loose, ```` ``` ```` a code block, so tight), and keystrokes would parse the page more than once.
- **Blocks into a list item.** `insert_block`, when the active block is a list item's own text, uses a new rule `edit::below_item_text`, run through `edit::nested` so quotes and outer items keep their prefixes: a pending paragraph right after the item's text, inside the item (`outer` gains `ContainerKind::Item` with the item's content column, as `into_previous_item` builds it), with a blank line before it. Tidying in `apply` then removes the blank lines again where the block allows it. Other active blocks keep `gap_pending`.
- **The editing rules keep their separators.** `enter_list_item`, `into_previous_item` and the pending paragraphs keep writing what they write today; tidying after them settles the form. This keeps the rules and their many tests as they are.

## Risks / Trade-offs

- [A list written loose on purpose elsewhere becomes tight when edited] → Chosen by the user: a list is loose only where it has to be.
- [While a block is typed in, the source can be mixed] → Settled when the block is left or the next command runs; the display follows the parser meanwhile, so a list that needs blank lines is already shown loose. A page saved while typing keeps the mixed source until the next edit of that list.
- [Extra parses] → One or two parses of the page per tidied list, only after commands and when leaving an edited block, never in `view`. Checked with the benchmark.
- [The parser check is too strict (some harmless difference)] → The list then stays loose or unchanged, never broken; tests cover code blocks, tables, quotes, headings, nested lists, rules, math and images.
- [Offsets in a rewritten separator] → Mapped to the separator's start; a pending paragraph's place is there, and the cursor is never inside a separator while a block is active.
