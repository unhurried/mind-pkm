# Proposal

## Why

The page is a column centered in the window, and a click in the empty space to its left or right did nothing: the cursor stayed where it was. Clicking there should work like pointing at the line beside it.

## What Changes

- A click in the margin left or right of the page's column puts the cursor at the start of the line at the height of the click, as if the line were clicked at its left edge. Where the line is not text (a gap between blocks, the room below the last block, a table, an image) the click acts like one at the column's left edge.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: clicking in the margins beside the page.

## Impact

- `src/app/editor/press_relay.rs` and the page column in `src/app/editor/view/document.rs`.
