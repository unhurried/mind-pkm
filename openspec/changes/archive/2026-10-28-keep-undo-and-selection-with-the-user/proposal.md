# Proposal

## Why

Exploratory testing found several ways in which the map's undo history, unsaved mark and selection went their own way:

- The automatic Next Steps refresh was an undo step: the next Ctrl+Z took back that invisible change instead of the user's last edit (issue #36).
- A refresh recreated all children of a page node whose Next Steps changed: checkmarks and other icons on them were lost, and a selected child left the selection on the root (issues #36, #35).
- Nodes added under a page node vanished with the next refresh without a word (issue #36).
- Undo and PKM Delete (F11) left the root selected, so the view jumped there (issue #35).
- Cancelling a new node kept the map marked as unsaved, and New Page / New Directory that failed added an empty undo step and marked the map unsaved (issue #34).

## What Changes

- The Next Steps refresh (automatic or F12) is no undo step: Undo and Redo take back the user's own changes and keep what the refresh did.
- Undo and Redo select the nodes that were selected at that point.
- When the selected node goes (Undo, Delete, F11, a refresh), the selection moves to its next sibling, else its previous sibling, else its parent.
- The refresh keeps the children of a page node whose text is still a Next Steps item (with their icons and attributes), deletes only the others and adds only the missing items.
- Adding, pasting or dropping a node under a page node shows a warning in the status bar.
- Cancelling a new node leaves the map's unsaved mark as it was; a command that changes nothing (New Page / New Directory / Delete that fail) adds no undo step and doesn't mark the map unsaved.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: undo and the refresh, the selection after undo and deletions, cancelling a new node.
- `task-management`: the Next Steps sync keeps matching children; a warning for nodes put under a page node.

## Impact

- `src/app/doc.rs` (undo history with selections, refresh applied to the history, selection fallback), `src/app/command.rs`, `src/app/dialog.rs`, `src/app/drag.rs`, `src/pkm/next_steps.rs`.
- Tests in `src/app/doc.rs`, `src/app/tests.rs` and `src/pkm/next_steps.rs`.
