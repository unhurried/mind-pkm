# Proposal

## Why

Shift+click on the map adds just the clicked node to the selection, one node at a time. Selecting a run of nodes - for example several siblings to move or delete them together - takes a click per node. Users expect Shift+click to select everything between two nodes, as in Freeplane and file managers.

## What Changes

- Shift+click selects the range from the anchor to the clicked node and replaces the selection with it. The anchor is the node focused before the first Shift+click; consecutive Shift+clicks keep it, so a later Shift+click widens or narrows the range from the same node.
- When the anchor and the clicked node are siblings, the range is the siblings between them (both included), without their descendants.
- Otherwise the range is every node shown on the map between them in document order (a node before its children, siblings in order); nodes hidden by a fold or the filter are not selected.
- The clicked node becomes the focused node. Ctrl+click and Shift+arrow keys are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: Selection and navigation (Shift+click no longer adds one node); new requirement Range selection with Shift+click.

## Impact

- `src/app/doc.rs` (`Doc::select_range` and the anchor it keeps), `src/app/mod.rs` (`node_clicked`), tests in `src/app/tests.rs`.
- README (the map's mouse table).
- No new dependencies; the `.mm` reader/writer and the document directory are untouched.
