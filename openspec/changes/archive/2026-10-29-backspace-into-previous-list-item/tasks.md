# Tasks

## 1. Implementation

- [x] 1.1 Backspace at the start of a list item's text: find the item it joins (previous sibling or parent)
- [x] 1.2 An empty item becomes a new empty paragraph inside that item; an item with text a paragraph of it
- [x] 1.3 Tables in list items and quotes: parse the table without its lines' prefixes, write it back with them; the cell at an offset after a prefix
- [x] 1.4 Tests: the editing rule (empty items, items with text, nested, in a quote, in a list item's blocks, cases left alone), the UI flow `* aaa`, Enter, Backspace, a table header row, Enter, a cell typed into; a table in a quote; cells at offsets after prefixes
- [x] 1.5 Run the app: `* aaa`, Enter, Backspace, `| bbb | ccc |`, Enter, a cell typed into, saved
- [x] 1.6 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
