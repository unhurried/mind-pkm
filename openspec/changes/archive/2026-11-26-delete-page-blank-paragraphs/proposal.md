## Why

The page view displays empty paragraphs that can be entered and typed into. Pressing Backspace or Delete in one moves the caret away and leaves the empty paragraph in the file.

## What Changes

- Delete one displayed empty paragraph with Backspace or Delete when its row holds the caret.
- Preserve the adjacent blocks and their Markdown separators.
- Record each deletion as one undoable edit.

## Capabilities

### Modified Capabilities

- `page-editor` gains deletion of empty paragraphs in the rendered page.

## Impact

Page-view editing, Markdown source range handling, and regression tests.
