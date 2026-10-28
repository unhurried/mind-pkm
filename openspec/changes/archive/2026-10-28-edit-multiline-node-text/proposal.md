# Proposal

## Why

Freeplane writes nodes with several lines of text. They were shown correctly on the map, but editing them used a single-line input that showed only the first line: End and typing acted on the invisible rest, and there was no way to see, add or remove lines (issue #32). The status bar's path of such a node kept its line breaks, so the status bar grew to several lines.

## What Changes

- The text dialog of a node (new node, Ctrl+E, F2 on a node that is neither a page nor a directory) is a field of several lines showing the whole text, with the cursor at its end. Enter confirms, Shift+Enter starts a new line.
- The status bar shows the selected node's path on one line, a node's lines joined by spaces.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: editing node text of several lines; the node path in the status bar.

## Impact

- `src/app/dialog.rs`, `src/app/view.rs`, `src/app/mod.rs` (the node text field), `src/pkm/mod.rs` (the node path).
- Tests in `src/app/tests.rs` and `src/pkm/mod.rs`.
