# Proposal

## Why

While a node's text is edited in the node, the node keeps the blue background of a selected node, which makes the text box look like a highlighted field. Moving a node into the node above it needs a drag and drop, although it is the usual keyboard action for building a hierarchy.

## What Changes

- A node whose text is being edited on the map has no background; it keeps the border of a selected node.
- Ctrl+Right makes the focused node the last child of the node above it (its previous sibling), unfolding that node. It is one undo step and does nothing for the root or a first child.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: Node decorations (no background while editing), Indenting a node.

## Impact

- `src/app/view.rs` (the edited node's style, the Edit menu), `command.rs`, `doc.rs`, `keys.rs`, `tests.rs`, `README.md`.
