# Proposal

## Why

There was no way to type a block (a table, a code block, a paragraph) under a list item. Enter at the end of an item starts a new item, and Backspace in it only removed its marker: the empty line left behind became a continuation line of the item above, so a table header typed there was part of that item's text, and Enter after it started yet another item instead of a table.

## What Changes

- Backspace at the start of the text of a list item that follows another item of its list, or that is the first item of a nested list, makes it a block of that item (the previous item, or the item its list is nested in) instead of removing its marker: an empty item becomes a new empty paragraph inside that item, ready for any block syntax (a table header row and Enter, a code fence, a quote...); an item with text becomes a paragraph of that item.
- The first item of a list at the top level keeps the current behavior: Backspace removes its marker, making it a paragraph.
- A table inside a list item or a quote keeps the item's indentation and the quote's `>` while its cells are edited. Until now editing a cell wrote the table back without them, moving a table out of its list item; a table in a quote could not be edited cell by cell at all.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: Backspace at the start of a list item.
- `table-editing`: editing a table inside a quote or a list item.

## Impact

- `src/markdown/edit.rs` (`backspace`), `src/app/editor/page.rs` (reading and writing the table being edited), `src/markdown/table.rs` (the cell at an offset of a prefixed row).
- Tests in `src/markdown/edit.rs`, `src/markdown/table.rs` and `src/app/editor/tests.rs`.
