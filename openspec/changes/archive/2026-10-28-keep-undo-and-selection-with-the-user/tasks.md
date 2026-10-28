# Tasks

## 1. Implementation

- [x] 1.1 Keep the selection and the unsaved / edited marks in the undo and redo steps; restore them on Undo, Redo and cancelling a new node
- [x] 1.2 Apply the refresh to the current map and the maps of the undo history instead of recording it
- [x] 1.3 Select the nearest node when the selected one is gone
- [x] 1.4 Reconcile the children of page nodes (and Due Today / Past Due) instead of recreating them
- [x] 1.5 Record New Page, New Directory and Delete (F11) only when they changed the map
- [x] 1.6 Warn in the status bar when nodes are added, pasted or dropped under a page node
- [x] 1.7 Tests: undo after a refresh, selections after undo and deletions, kept children with icons, failed New Page, cancelled new node, the warning
- [x] 1.8 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
