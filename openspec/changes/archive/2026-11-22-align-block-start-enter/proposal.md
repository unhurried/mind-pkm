# Proposal

## Why

Block editing should keep the cursor with the text after a split and preserve the document structure. Enter currently moves the cursor above ordinary text, separates completed tasks from their checkboxes, and can remove the rendered form of front matter and display math.

## What Changes

- Match Typora at the start of ordinary paragraphs, including those in quotes.
- Preserve completed task status on the parts that still contain the original text and write sequential markers when splitting ordered items.
- Show leading indented blank lines in the editor's code block while keeping the source indented.
- Keep front matter and display math recognized when Enter adds blank lines inside their delimiters.

## Capabilities

### Modified Capabilities

- `page-editor` defines text splits, list and code editing, and Enter inside existing delimited blocks.
- `markdown-rendering` defines how front matter and display math with blank lines are interpreted.

## Impact

The Markdown parser, editing rules and editor layout share the new behavior. Saved pages retain plain Markdown.
