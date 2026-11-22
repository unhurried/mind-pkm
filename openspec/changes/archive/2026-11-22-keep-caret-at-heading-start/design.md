# Design

## Context

The editor stores the Markdown source and source offsets for the caret and for undo. CommonMark skips blank lines when producing blocks. The editor currently shows a new empty paragraph while it has the caret.

## Decisions

- Insert two source line breaks before the heading and map the caret into the preserved heading. The existing nested edit path adds the quote prefixes and retains the item's container. Source line breaks make the edit available to Save and Undo.
- Show pairs of blank lines immediately before a heading as editable rows in the page column. Ordinary block separation occupies at most one blank line. Reuse the existing empty paragraph editor when the user clicks a row or moves into it with the arrow keys.
- Preserve these paragraphs when formatting list separators, and use loose spacing elsewhere in a list that contains one. Keep the Markdown parser's block model for rendering, export, and whole-block selection.

## Risks / Trade-offs

Existing Markdown with multiple consecutive blank lines immediately before a heading can show extra empty paragraphs in the rendered editor. The same whitespace convention makes the rows available after saving and reopening.
