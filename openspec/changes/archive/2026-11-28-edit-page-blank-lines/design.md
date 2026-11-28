## Context

The page editor stores Markdown and byte offsets. Its rendered column adds editable rows for whitespace that the Markdown parser leaves between blocks.

## Decisions

- Discover individual source lines between parsed blocks, including both ends of the document. Determine their containers from the neighboring blocks and the source prefixes. Reserve the separators needed by the parsed blocks and the spacing of loose lists.
- Share the row ranges across layout, caret placement, keyboard navigation, and editing. Keep temporary insertion points for container exit commands.
- Use ordinary Markdown whitespace, local source edits, and the existing undo history. Treat extra lines as user content when normalizing an edited list.
