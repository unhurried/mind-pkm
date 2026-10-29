# Proposal

## Why

Several blocks could be selected only from the keyboard (Shift+Up/Down, Ctrl+A twice) or with Shift+click. Dragging the mouse from one block to another, the usual way of selecting a stretch of a document, selected nothing beyond the block the press started in.

## What Changes

- Pressing in a block and dragging onto another block selects the blocks from the one the press started in to the one under the pointer, following the pointer until the button is released (outside source mode).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: selecting several blocks by dragging.

## Impact

- `src/app/editor/mod.rs` (the block a press started in, the pointer entering a block, the release), `src/app/editor/view.rs` (rendered blocks report the pointer entering them).
- A test in `src/app/editor/tests.rs`.
