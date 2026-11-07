# Tasks

## 1. Moving list items

- [x] 1.1 The rule: swap a list item's subtree with its previous / next sibling, numbers kept in place (width changes re-indent the item's lines), the selection mapped. Verify with `edit.rs` tests (tasks, nested items, blocks under items, ordered numbers 9/10, first/last item, nested in a quote, not a list item).
- [x] 1.2 Commands Move Item Up / Down in the Paragraph menu with Ctrl+Up / Ctrl+Down, run on the active region (inside its containers) and in the source mode. Verify with the `keys.rs` tests and editor tests driving Ctrl+Up / Ctrl+Down.
- [x] 1.3 README: the editor's shortcut table. Verify that the `keys.rs` tests still agree with the README.
