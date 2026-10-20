# Proposal

## Why

Folding a node with the mouse is only possible one way: the child count next to a folded node unfolds it, but there is nothing to click to fold an unfolded node. The point where the edges to a node's children branch is the natural place for it.

## What Changes

- Clicking the branch point of the edges between a node and its children folds the node. Hovering it shows a small circular marker, so it can be found.
- After folding, the child count shown next to the node (as before) unfolds it again.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: the folding requirement adds folding by clicking the branch point; the drag-to-pan requirement lists it among the presses that keep their own behavior.

## Impact

- `src/app/view.rs`: a clickable handle at each unfolded node's branch point.
