# Proposal

## Why

Moving around a large map currently needs the scrollbars, the mouse wheel or the keyboard. Grabbing the map and dragging it - as in Freeplane and most map viewers - is the quickest way to bring another part into view.

## What Changes

- Pressing the left mouse button on an empty part of the map and dragging scrolls the map with the pointer, horizontally and vertically.
- The pointer shows an open hand over empty map space and a closed hand while dragging.
- Clicks on nodes, link markers, fold markers and scrollbars keep their current behavior; a drag that starts on them does not pan.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: adds the requirement that the map can be panned by dragging empty space.

## Impact

- The map view (`src/app/view.rs`) and the application state and messages (`src/app/mod.rs`).
- No change to `.mm` files, the document directory or the search index.
