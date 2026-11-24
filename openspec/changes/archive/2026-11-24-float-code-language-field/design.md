## Context

The table toolbar already uses an overlay whose layout is independent of its content. The code language field is in the code editor's layout.

## Goals

- Keep the code box and following blocks in place while the language field appears or disappears.
- Keep the field editable and anchored near the right edge of its code box.

## Decisions

Extend the shared overlay with a bottom-right placement. Keep its bounds within the visible page area and use the code background to keep the text legible over the page. Follow the table toolbar's visibility rules for menus and dialogs. Preserve the code body's trailing blank lines in rendered mode so the code box has the same height in both modes.

## Risks and Trade-offs

The field can cover nearby text while active. Its compact width limits the area covered.
