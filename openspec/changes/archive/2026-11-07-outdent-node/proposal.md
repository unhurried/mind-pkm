# Proposal

## Why

Ctrl+Right makes a node a child of the node above it, but there is no keyboard action for the opposite step. Moving a node up a level needs a drag and drop.

## What Changes

- Ctrl+Left (Edit → Make Sibling of Parent) moves the focused node, with its subtree, up one level: it becomes the next sibling of its parent. It is one undo step and does nothing for the root and for a child of the root.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: Outdenting a node.

## Impact

- `src/app/command.rs`, `doc.rs`, `keys.rs`, `view.rs` (the Edit menu), `tests.rs`, `README.md`.
