# Proposal

## Why

Pasting from VS Code or a browser into a code block inserted Markdown (`let a\_b = c\*d;`, with blank lines between the lines): the HTML on the clipboard was converted to Markdown whenever it was there, also inside code (issue #25). Code is not Markdown, so its escapes and blank lines are wrong there.

## What Changes

- Ctrl+V inside a code-like block (code block, math, front matter, HTML block) inserts the clipboard's plain text as it is; the HTML flavor is not converted.
- When the clipboard holds HTML only, its text is inserted (tags dropped, line breaks kept, entities decoded).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: the clipboard requirement.

## Impact

- `src/app/editor/mod.rs` (paste), `src/markdown/html.rs` (HTML to plain text).
- Tests in `src/app/editor/tests.rs` and `src/markdown/html.rs`.
