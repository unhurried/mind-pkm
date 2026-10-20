# Proposal

## Why

Cancelling the text dialog of a new node (Tab / Enter) undoes the last recorded change, assuming it is the node's creation. When the background Next Steps refresh changes the map while the dialog is open, the cancel undoes the refresh instead and leaves the empty node in the map. Undoing also hands out node numbers again that nodes undone away had, so a reference to such a node taken before the undo (a search result's node, an open dialog) can end up pointing at a different node.

## What Changes

- Cancelling a new node's text dialog removes exactly that node, keeping every change made to the map meanwhile; no undo step brings the empty node back.
- Nodes created after an undo never reuse the identity of a node that existed before it, so references to nodes held across an undo never point at another node. (Internal; nodes' Freeplane IDs are unaffected.)

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: Structural editing gains a scenario for cancelling a new node while the map changes in the background.

## Impact

- `src/app/doc.rs`: the undo history (entries identified per checkpoint), cancelling a new node, restoring older maps.
- `src/mindmap.rs`: keeping node numbers from being handed out again.
- `src/app/dialog.rs`, `src/app/command.rs`: the new node dialog refers to the checkpoint of the node's creation.
