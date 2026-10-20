# Proposal

## Why

A node with a single child of the same height is centered on it, so the edge between them is one straight line and the two nodes read like one long underline; parent and child are hard to tell apart.

## What Changes

- A node's only shown child is placed a little higher than its parent (8 px at 100% zoom, scaled with the zoom), so the edge bends up into the child.
- Nodes with two or more shown children stay centered on them.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: the right-only layout requirement says where a single child goes.

## Impact

- `src/app/layout.rs`: placing a node's subtree and its height.
