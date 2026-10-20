# Proposal

## Why

The mouse pointer changes shape over the window: a pointing hand over nodes and buttons, an I-beam over text boxes, open and closed hands over the map while panning. The pointer should simply stay the normal arrow.

## What Changes

- The pointer is the normal arrow everywhere in the window: over the map, nodes, buttons, menus, text boxes and drop-down lists, and while dragging the map.
- Panning by drag keeps working; only its hand pointers go.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `application-shell`: adds the requirement that the pointer is always the arrow.
- `mindmap-editing`: "Drag to pan" no longer shows hand pointers.

## Impact

- The root of the view (`src/app/view.rs`) is wrapped in a widget that reports the arrow for every pointer query, including pop-ups (`src/app/arrow_cursor.rs`).
