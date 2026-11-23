## Context

The rendered editor holds either a selection within the active block or a range of whole blocks. Parsed lists provide the bounds and depth of each group of siblings.

## Goals / Non-Goals

Keep list depth changes local to the target items. Use the same keyboard entry points for text and whole-block selections.

## Decisions

- Find siblings within the same parsed list and container. For a caret in a parent item, reuse the list move rule to move its first child past the next sibling before nesting it. Moving past that sibling is necessary because the first child has no preceding item to nest under. A leaf or an item with text selected uses its previous sibling. Whole-block selections use the first and last selected items as their bounds.
- Shift the source lines after any surrounding quote or item prefixes. For a whole-block Tab, stop before the first unselected descendant of the last item. Apply the result as one edit and restore the whole-block selection from its mapped endpoints.
- Shift+Tab uses the active item or selected range. Code indentation and table navigation use their existing bindings.

## Risks / Trade-offs

A whole-block selection can span different lists. The range can end in a descendant within the starting list, provided the first item has a preceding sibling. A trailing unselected item keeps its source indentation, with list spacing normalized as usual.
