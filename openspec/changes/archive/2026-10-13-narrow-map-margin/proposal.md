# Proposal

## Why

The map is surrounded by empty space of more than half the viewport on every side, so there is a lot to scroll past and the scrollbars are much longer than the map needs.

## What Changes

- The empty space around the map is a narrow fixed margin (40 px) instead of 60% of the viewport.
- Centering a node (Esc, opening a map, Select Node, node links) goes only as far as scrolling allows, so a node near an edge of the map stays near that edge of the view.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: adds the requirement on the margin around the map.

## Impact

- `src/app/viewport.rs`: the margin passed to the layout.
