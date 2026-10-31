# Proposal

## Why

Moving the cursor into a heading (Right, Down or Up from a neighbouring block, a click beside its text, leaving a block selection with an arrow key) put it at the start of the block's source, before the `###` the block does not show. Typing there produced `bbb ### aaa` instead of `### bbb aaa`.

## What Changes

- Whenever the cursor is placed in a block edited rendered, a cursor in the block's hidden syntax (a heading's `#`s, a list item's marker, a quote's `>`) moves to where the block shows it: the start of its text.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: a new requirement on a cursor placed in a block's hidden syntax.

## Impact

- `src/app/editor/page.rs` (placing the cursor in a live block).
- Tests in `src/app/editor/tests/editing.rs`.
