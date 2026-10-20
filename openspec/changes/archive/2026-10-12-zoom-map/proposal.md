# Proposal

## Why

The map is always drawn at one size. On a large map it is hard to get an overview, and on a high-resolution screen the text can be small. Zooming - as in Freeplane, browsers and most map viewers - solves both.

## What Changes

- Ctrl+Plus zooms the map in, Ctrl+Minus zooms it out and Ctrl+0 resets it to 100%, in fixed steps from 50% to 300%.
- Ctrl+mouse wheel zooms in (wheel up) and out (wheel down), keeping the point under the pointer in place; without Ctrl the wheel keeps scrolling.
- Keyboard zooming keeps the selected node in place on screen.
- A View menu lists Zoom In, Zoom Out and Reset Zoom; the status bar shows the zoom level.
- Zooming changes only how the map is shown: not the map file, the undo history or the unsaved state. The zoom is kept when another map is opened and starts at 100% on every start.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: adds the zoom requirement; the compact spacing figures are for 100%.
- `application-shell`: adds the View menu to the menu bar.

## Impact

- The map view and layout (`src/app/view.rs`, `src/app/layout.rs`, `src/app/viewport.rs`), commands, key bindings and menus, a wrapper widget catching Ctrl+wheel over the map.
- No change to `.mm` files, the document directory or the search index.
