# Proposal

## Why

After a table cell gains focus, pressing text in another cell can move the caret outside the table. A small pointer movement during the press can then highlight the table and a neighboring block.

## What Changes

- Clicking another cell places the caret in that cell and clears the previous selection.
- Dragging from a newly activated cell continues to select a range of cells.

## Capabilities

### Modified Capabilities

- `table-editing` covers clicking and dragging between cells in the active table.

## Impact

The page editor's table rendering and pointer handling will change. A widget-level regression test will cover the click sequence.
