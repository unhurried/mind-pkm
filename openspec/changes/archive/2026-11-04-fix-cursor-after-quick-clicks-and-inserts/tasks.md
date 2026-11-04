# Tasks

## 1. Cancelling a new node

- [x] 1.1 `cancel_new_node` selects again the selection of the checkpoint before the node (nodes still there, else the parent), also when changes landed meanwhile; the node edit selects nothing itself
- [x] 1.2 Tests: Enter then Esc, and Enter then an empty text, select the node the sibling was added after; the same after a refresh landed meanwhile

## 2. Insert Table

- [x] 2.1 `insert_block_with_cursor`; Insert Table with the cursor in the first header cell
- [x] 2.2 Test: Ctrl+T then typing fills the first header cell, also for a table inserted in a list item

## 3. Quick clicks

- [x] 3.1 `PressRelay::tracking` publishes `Released` for the release of a left press it saw
- [x] 3.2 Test: a press and a release on a page's text in one batch of events leave no button held; a paragraph turned into a math block by typing keeps the cursor
- [x] 3.3 Run the app: a quick click into `$$` / `\frac{1}{0` / `$$`, End, `}X` gives `\frac{1}{0}X`; a quick click, Home, Ctrl+click on a link selects nothing

## 4. Checks

- [x] 4.1 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
