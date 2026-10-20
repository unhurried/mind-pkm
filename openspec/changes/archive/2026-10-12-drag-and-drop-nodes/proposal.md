# Proposal

## Why

Moving a node to another place in the map currently needs cut and paste, or repeated Ctrl+Up / Ctrl+Down within its siblings. Dragging the node to where it belongs - as in Freeplane - is the direct way.

## What Changes

- Pressing the left button on a node and dragging it onto another node moves the node there: onto the middle of the target as its last child, onto the top or bottom edge of the target as the sibling before or after it.
- With several nodes selected, dragging one of them moves all selected nodes, in map order (a press on a selected node keeps the selection; a click without dragging still selects just that node).
- While dragging, the drop position is shown: an outline around the target for a child, a line above or below it for a sibling.
- A drop onto a dragged node or its descendants, or away from any node, changes nothing; Esc or leaving the window cancels the drag.
- Moved nodes keep their IDs; the move is one undo step.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: adds the requirement that nodes can be moved by drag and drop.

## Impact

- The map view, the application state and messages, and the open map's edits (`src/app/`).
- No change to the `.mm` format: a move is the same change as cut and paste, except that the nodes keep their IDs; children moved under the root get the right-side POSITION.
