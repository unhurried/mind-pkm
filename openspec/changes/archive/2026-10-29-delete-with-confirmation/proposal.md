# Proposal

## Why

The Delete key removed the selected nodes at once, without asking and without touching their files, while Delete (F11) asked first, moved the files of pages and directories to the Recycle Bin and refused any other node. Two commands for one gesture made it easy to leave files behind, or to lose nodes with no chance to back out.

## What Changes

- The Delete key runs the same command as F11: a confirmation listing the selected nodes, then pages and directories go to the Recycle Bin with their files. Nodes that are neither a page nor a directory are deleted as nodes (no longer refused).
- Holding Ctrl (Ctrl+Delete, Ctrl+F11) deletes without the confirmation.
- The Edit menu's "Delete Node" entry is replaced by the PKM menu's Delete (shown as "F11 / Delete") and a new "Delete without Confirmation" entry.
- **BREAKING**: the Delete key no longer deletes without asking, and no longer leaves the files of pages and directories in place.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document-management`: Delete (F11) also takes the Delete key and accepts plain nodes; Ctrl skips the confirmation.
- `mindmap-editing`: structural editing no longer has a separate Delete without touching files.

## Impact

- `src/pkm/commands.rs` (targets may be plain nodes), `src/app/command.rs`, `src/app/keys.rs`, `src/app/view.rs`, `src/app/dialog.rs`, `src/app/doc.rs` (the old node-only delete goes), README.
- Tests in `src/pkm/commands.rs`, `src/app/keys.rs`, `src/app/tests.rs`.
