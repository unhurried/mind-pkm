# Tasks

## 1. Segmentation (`src/markdown/parse.rs`)

- [ ] 1.1 Containers and `outer`: every block of a quote and the later blocks of a list item become blocks of their own, starting at their first line; empty quotes are `Quote` blocks; container-only lines count as blank; verify with tests for a list, heading and quote in a quote, code in a list item, continuation after it, and ranges
- [ ] 1.2 Parsing with a masked byte (text events get the real text); verify that ` ``` ` masked leaves the following blocks alone and is shown as typed

## 2. Rules (`src/markdown/edit.rs`, `format.rs`)

- [ ] 2.1 `nested`: rules applied inside the containers' prefixes, results mapped back with canonical prefixes on changed lines, selection and new paragraphs mapped; verify Enter, Shift+Enter, Backspace, Delete and Tab in a list in a quote and in a quoted paragraph
- [ ] 2.2 New paragraphs inside containers (and in gaps inside them); Enter in an empty one leaves the container; Backspace at the start of a container's first block leaves it; verify each scenario of "Enter in lists, quotes and code" and "Backspace and Delete across blocks"
- [ ] 2.3 Heading and list shortcuts inside containers

## 3. Editor

- [ ] 3.1 Undetermined markers in `Page` (mask after live edits, dropped on leaving, rules, undo); verify each scenario of "Converting syntax as it is typed" in the editor app tests
- [ ] 3.2 View: quote bars and item indentation around blocks, the active block and new paragraphs, continuous across the gaps; display math paragraphs edited rendered
- [ ] 3.3 README and CLAUDE.md; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (also for the Windows target), `cargo test`, `openspec validate --all --strict`; run the app under Xvfb and look at typing markers, lists in quotes and code in list items
