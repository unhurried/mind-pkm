## Context

The page view discovers displayed empty paragraphs from pairs of blank source lines before a heading or paragraph. Lists also use this detection to preserve empty paragraphs during spacing normalization. A caret in a displayed empty paragraph uses the same active editor as an unwritten paragraph.

## Decisions

- Return the pair's source range alongside its caret offset. Reuse the detection for layout, navigation, and deletion so all three refer to the same displayed row.
- Handle deletion while the caret is in a source-backed empty paragraph. The existing pending-paragraph behavior remains the fallback for an unwritten paragraph.
- Use the existing undoable source edit path that preserves the rest of the source byte for byte. Removing the pair leaves the separator used by the adjacent blocks. Generic list normalization could rewrite other separators in the list during this operation.
