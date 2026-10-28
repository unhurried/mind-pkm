# Proposal

## Why

Two kinds of paste turned into a table they shouldn't. Any plain text with a tab and two lines became a table, such as tab-indented code (issue #24). And HTML holding a `<table>` anywhere, such as part of a web page, was pasted as that table alone: the paragraphs and lists around it were dropped (issue #26).

## What Changes

- Tab-separated text becomes a table only when it reads as a grid: every row has the same number of cells, and not every row starts with a tab.
- HTML that holds a table besides other content is pasted as Markdown in full: the content around the table as converted Markdown, the table as a Markdown table, in their order. HTML that is only a table (a spreadsheet's) still pastes as cells.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `table-editing`: what pasted text or HTML becomes a table.

## Impact

- `src/markdown/table.rs` (the TSV and HTML table parsing), `src/app/editor/mod.rs` (paste).
- Tests in `src/markdown/table.rs` and `src/app/editor/tests.rs`.
