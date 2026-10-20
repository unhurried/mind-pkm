# Proposal

## Why

The vertical spacing between nodes still feels loose; more of the map should fit on the screen.

## What Changes

- Node texts and icons use a tighter line height (1.2 times the font size instead of 1.3), and a node with an icon is no taller than one without.
- Sibling subtrees are 1 px apart instead of 2 px.
- A single-line row takes about 22 px at 100% zoom instead of about 25 px.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: the compact spacing requirement gets the new numbers.

## Impact

- `src/app/view.rs`: node line height, node height.
- `src/app/layout.rs`: the vertical gap.
