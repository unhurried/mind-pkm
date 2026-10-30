# Proposal

## Why

Two things went wrong with a list item that has other blocks below its text, such as a code block:

- Enter at the end of the item's text started a new item that took over those blocks. An empty item cannot hold blank lines before its first block, so the code block became the new item's first block, and the empty item was shown and edited as that code block.
- Down at the last line of a code block ending the item went on into the block after the list. There was no way to add another block to the item after the code from the keyboard.

## What Changes

- Enter at the end of a list item's text keeps the blocks below it (paragraphs, code, quotes, ...) in that item and starts the new item after them. Nested items still move under the new item, as before.
- Down at the last line of a code-like block that is the last block of a list item or quote starts a new empty paragraph inside that item or quote; Down from that empty paragraph goes on to the next block.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: new requirements for Enter after the text of a list item with blocks below it, and for Down below a code block ending a list item or quote.

## Impact

- `src/markdown/edit.rs` (Enter in a list item), `src/app/editor/page.rs` and `src/app/editor/mod.rs` (Down below code ending a container).
- Tests in `src/markdown/edit.rs` and `src/app/editor/tests/editing.rs`.
