# Design

## Context

The editor stores source offsets through edits, saving, and undo. Empty text paragraphs and leading indented code lines need extra representation because ordinary CommonMark events skip them. Blank lines can also divide a math expression into separate paragraphs.

## Decisions

- Reuse the editable whitespace rows already used before headings for ordinary paragraphs and quotes. Keep their Markdown separators through list normalization.
- Keep checkbox state with the original task text. Renumber markers after an ordered split using the parsed list structure, keeping nested indentation aligned when marker widths change.
- Extend the editor's indented code range through explicitly indented leading blank lines. Code exports continue to use CommonMark events.
- Keep the opening delimiters of existing YAML and math blocks intact on Enter. Recognize YAML mappings with a blank first line and restore standalone display math across blank lines within the same container, with original offsets and TeX.

## Risks / Trade-offs

An authored Markdown file with consecutive blank lines before a paragraph may now show additional editable rows. Standalone math delimiters can span multiple elements, so the editor reparses those pages as a whole.
