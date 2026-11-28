## Why

The page view combines consecutive Markdown blank lines into empty paragraphs and omits them at some block boundaries. Enter also cannot add a line while the caret is already in a top-level blank row.

## What Changes

- Display and navigate extra Markdown blank lines individually, including at the document edges and beside any block.
- Insert and delete one blank row at a time, preserve required Markdown separators, and restore the rows on reopening.
- Retain quote and list structure when editing blank rows.

## Capabilities

### Modified Capabilities

- `page-editor` defines blank-row display, navigation, editing, and persistence.

## Impact

The page column, Markdown editing and list spacing rules, and editor regression tests.
