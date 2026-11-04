# Tasks

## 1. Backspace in an empty task item

- [x] 1.1 `backspace` in `src/markdown/edit.rs`: an empty task item loses its box and bullet at once (into the item before, or a paragraph of its own); verified by `edit.rs` tests (`- [ ] `, `- [x] `, `1. [ ] `, in a quote, after an item, nested) and the editor test in `src/app/editor/tests/lists.rs` (Enter after a task, Backspace, typing)
