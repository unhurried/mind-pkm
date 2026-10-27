# Proposal

## Why

Moving a node up or down among its siblings (Ctrl+Up / Ctrl+Down) keeps the moved node in place on screen, so the whole map scrolls by one sibling's height on every press. The user wants the map to stay where it is while the moved node is visible, and to scroll only by as much as needed when the node would leave the view.

## What Changes

- Ctrl+Up / Ctrl+Down no longer scroll the map to keep the moved node in place: the rest of the map stays where it is on screen and the node moves.
- When the moved node would leave the visible area, the map scrolls just enough to bring it back into view.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: a new requirement on how the view follows a node moved among its siblings.

## Impact

- `src/app/mod.rs`, `src/app/viewport.rs`, `src/app/command.rs`: the node kept in place after a move among siblings is the parent instead of the moved node.
- Tests in `src/app/tests.rs`.
