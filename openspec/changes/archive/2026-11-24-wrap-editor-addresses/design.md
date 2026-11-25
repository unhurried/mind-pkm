## Context

Rendered paragraphs and table cells use iced's rich text layout. The active block and cell use a separate layout that also positions the caret and selection.

## Goals

Use the available width before wrapping an address. Keep link targets, click positions, and editing offsets tied to the original Markdown.

## Decisions

- Identify visible URLs and the Markdown destinations revealed while editing inline links.
- Give the rendered text layout break opportunities at character boundaries inside an address. Map its display offsets back to the original text for pointer interactions.
- Allow the same breaks in the active layout, including through URL punctuation. Use the text area in the page and the inner width in a cell.

## Risks / Trade-offs

URL punctuation and find highlighting can split the styled text into runs, so verification covers the rendered geometry and source positions in addition to the active layout.
