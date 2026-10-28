# Proposal

## Why

Esc is to center the selected node, but it did nothing for the root, for nodes near the top or left of the map, or when the whole map fit in the window; dragging empty map space didn't move such a map either (issue #31). The scrollable area is the map plus a narrow 40 px margin (chosen so that there is little empty space to scroll past), and a scroll offset can't go below zero, so there was no room to move such a node to the middle.

## What Changes

- Esc centers the selected node wherever it is: room is added around the map on the sides where it is needed.
- Dragging empty map space moves the map with the pointer past its edges too, also when the map fits in the window.
- Room added this way goes again once it is scrolled out of view, so the scrollable area is back to the map plus its narrow margin.
- Opening a map, following a node link and Select Node keep centering only as far as the map area allows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: Esc centering, panning past the edges, the margin around the map.

## Impact

- `src/app/viewport.rs` (room around the map, free centering, panning, trimming), `src/app/layout.rs` and `src/app/doc.rs` (margins on each side), `src/app/mod.rs`, `src/app/file.rs`, `src/app/command.rs`.
- Tests in `src/app/tests.rs`.
