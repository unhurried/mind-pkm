# Design

## Context

Node boxes are mouse areas that select on press; the map is a scrollable stack wrapped in a mouse area that starts a pan on presses no node captured.

## Decisions

- **Starting**: a press on a node selects it as before and arms a drag of the selected nodes (the root excluded) if the pressed node is still selected afterwards (a Ctrl+click that unselects it arms nothing). A plain press on a node of a multiple selection keeps the selection (only moving the focus) and selects the node alone on a release without a drag, as in Freeplane. The drag starts only after the pointer moved more than a few pixels from where it was first seen, so clicks and double-clicks are unaffected.
- **Pointer in map coordinates**: a mouse area inside the scrollable, around the node stack, reports moves in content coordinates (scrolling included) - only while a drag is armed, so ordinary movement costs nothing. A subscription, active only then, ends the drag on button release and cancels it when the pointer leaves the window.
- **Drop position**: the node box under the pointer is the target; its top quarter means "before", its bottom quarter "after", the rest "child". The root only takes children. A target among the dragged nodes or under one of them gives no drop. This is a pure function of the layout and the map, unit-tested.
- **Moving**: before/after insert next to the target in its map parent (not the filtered layout parent), keeping the dragged nodes' map order; a child drop appends and unfolds the target so the moved nodes stay visible. A drop that leaves every node where it was is not recorded for undo and does not mark the map changed.
- **Feedback**: an outline around the target, or a 3 px line at its top or bottom edge, drawn on top of the nodes; those widgets do not take input.

## Risks / Trade-offs

- No auto-scroll at the viewport edge: a target outside the view is reached by scrolling with the wheel while dragging.
- Ctrl+drag (copy in Freeplane) is not supported; copy and paste covers it.
