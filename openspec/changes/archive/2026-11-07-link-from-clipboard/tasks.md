# Tasks

## 1. Link destination from the clipboard

- [x] 1.1 The link editing rule takes an optional destination; a function tells whether a clipboard text is a URL or a path and gives the destination to write. Verify with unit tests (URLs, Windows/UNC/Unix/relative paths, quoted paths, plain text, multi-line text, unbalanced parentheses).
- [x] 1.2 Ctrl+K reads the clipboard's text in the background and inserts the link when it arrives, in the active block and in the source mode. Verify with editor tests (a URL with and without a selection, a path, other text).
- [x] 1.3 README and the page-editor spec: Ctrl+K takes a URL or a path from the clipboard. Verify `openspec validate --all --strict`, and look at the app under Xvfb.
