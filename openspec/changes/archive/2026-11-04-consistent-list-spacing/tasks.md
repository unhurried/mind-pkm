# Tasks

## 1. The lists of a page

- [x] 1.1 `Document::lists` in `src/markdown/parse.rs`: each list (nested ones too) with its range, depth, containers around it and its separators (the start of the block after each); verified by `parse.rs` tests for a tight list, a loose list, a nested list, blocks inside items (code, quote, paragraph, tight text after a nested list) and a list in a quote

## 2. Writing a list tight or loose

- [x] 2.1 `src/markdown/lists.rs`: rewrite one list's separators tight or loose (keeping a blank line's `>`s, leaving separators with other content alone) and the `Shift` of offsets; verified by unit tests of both forms, inside a quote, and offset mapping
- [x] 2.2 The parser check and `tidy(text, changed)`: tight when the tight candidate parses to the same blocks, else loose when the loose one does, else unchanged; every list overlapping or touching the changed range, one at a time; verified by unit tests: code block, table, quote, heading, nested list and `***` keep a list tight; a paragraph, an image line, `$$` math and `---` make it loose; a mixed list is unified; a list away from the change is untouched; nested lists take their own form
- [x] 2.3 `edit::block_below_item`: a pending paragraph right below a list item's text, inside the item, also through `edit::nested` in a quote; verified by `edit.rs` tests

## 3. The page editor

- [x] 3.1 `Page::apply` tidies around the change (outcomes without a pending paragraph) in the same undo step and maps the selection; other edits and a pending outcome remember an untidy edit; verified by editor tests for Enter in a mixed list and Backspace of an item with text into the item before
- [x] 3.2 `deactivate` tidies around the block left when an untidy edit is remembered, after `collapse_blank_lines`, and returns the `Shift` that `place` applies to its position; verified by editor tests: a paragraph typed under an item makes the list loose on leaving it, deleting it makes the list tight again, the cursor lands in the clicked block
- [x] 3.3 Inserting a block (code, math, rule, table, pasted cells) while a list item's text is active goes into the item; inserting a block, deleting selected blocks and pasting over them go through `apply`; verified by editor tests: Ctrl+Shift+K and Ctrl+T in a tight list keep one tight list, a rule makes it loose, an ordered list keeps its numbers
- [x] 3.4 The code block typed under an item (Enter, Backspace, ```` ``` ````, Enter, `x`) leaves the list tight; find and replace, source mode and undo do not tidy; verified by editor tests

## 4. Specs, docs and verification

- [x] 4.1 CLAUDE.md (Structure: `lists.rs`; the editor tidies the lists it edits); check that the existing editing tests still pass unchanged or are updated only where the spec changed
- [x] 4.2 Run the app under Xvfb: put a code block, a table and a paragraph under list items and look at the spacing; save and check the source
- [x] 4.3 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`, and the benchmark (`perf`) shows no regression

## Workflow follow-up

- Archive the change with `/opsx:archive` after it is implemented, merging the deltas into `openspec/specs/`.
