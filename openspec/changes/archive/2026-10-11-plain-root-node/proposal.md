# Proposal

## Why

The root node is drawn in a framed, shaded box with more padding than other nodes. Now that every node uses the same font size and the map grows only to the right from the root, the frame just adds visual weight; the root should look like any other node.

## What Changes

- The root node is drawn exactly like other nodes: no frame, no background, the same padding and height.
- Selection highlighting is unchanged and applies to the root like to any node.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: the "Node decorations" requirement no longer draws the root in a box.

## Impact

- Node rendering and node size measurement (`src/app/view.rs`).
- No change to `.mm` files.
