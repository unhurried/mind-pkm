## Why

Making a link with Ctrl+K usually starts with copying its URL or the path of a file: after Ctrl+K the user pastes it between the parentheses by hand. When the clipboard already holds a URL or a path, Ctrl+K can put it in itself.

## What Changes

- Ctrl+K (Format → Link, and Link in the right-click menu) looks at the text on the clipboard. When it is a URL or a file path, the link is inserted with it as its destination: `[selection](url)` with the cursor after the link, or `[](url)` with the cursor in the link text when nothing is selected.
- Otherwise Ctrl+K inserts the link as before, with the cursor in the empty destination.
- A path copied with Windows Explorer's "Copy as path" (in double quotes) counts, without its quotes; the backslashes of a Windows path are written as `/`, so that Markdown does not take them as escapes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: Ctrl+K moves from the format shortcuts requirement into a requirement of its own, inserting links, which says what it inserts when the clipboard holds a URL or a path.

## Impact

- The link editing rule (a destination given), a check of the clipboard text, reading the clipboard's text for Ctrl+K in the background, editor tests, README.
