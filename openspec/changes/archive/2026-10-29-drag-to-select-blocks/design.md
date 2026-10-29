# Design

## Context

A rendered (inactive) block is a mouse area that reports presses. The active block is edited in its own widget, which turns a drag into selection messages within the block. Dragging over table cells already ends on a button release seen anywhere in the window (a subscription).

## Decisions

- **The anchor is the block the press left active.** After any press outside source mode (rendered text, beside a block, a task box or link, a cell, the active block's own text), the editor remembers the active block, or the anchor of the block selection after Shift+click, until the left button is released. Shift+drag thus extends a Shift+click selection.
- **Entering a block selects.** Rendered blocks report the pointer entering them. While a press is held, entering a block other than the anchor selects the blocks from the anchor to it; once blocks are selected, every block entered (the anchor too) moves the selection's end. The active block's widget does not capture pointer moves, so the blocks around it see the pointer arrive during a text drag.
- **The release ends it anywhere.** The window-wide release subscription that ends a cell drag now also ends a block drag (one message for both).

## Risks / Trade-offs

- A drag back into the anchor keeps that one block selected rather than returning to a text selection in it; returning would need the active block back under the held button.
- The page does not scroll by itself while dragging past its top or bottom edge; the wheel still scrolls during the drag.
