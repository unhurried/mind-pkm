## Context

The outline lists `Document::headings` (level, text, anchor, block) in a scrollable column beside the page. The assets pane already folds its folder tree with ▸ / ▾ arrows.

## Decisions

- A heading's children are the headings after it up to the next heading of the same or a higher (smaller) level, so a jump in levels (H1 then H4) still nests.
- Folded by default: a heading of level 3 or more. The window keeps only the entries the user toggled, keyed by the heading's anchor (unique on a page, `-1`, `-2`... for repeats), as the state opposite to the default. Indices and block numbers shift with every edit above a heading; the anchor changes only when the heading's own text does, and then the heading falls back to its default.
- Which entries are shown is a pure function of the headings and the toggled anchors (`outline.rs`), so it is tested without a window; the view only draws its result.
- The toggles are not saved: a newly opened window starts from the default.
