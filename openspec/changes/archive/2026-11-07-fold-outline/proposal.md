## Why

On a long page the outline lists every heading down to H6, so the structure is hard to see at a glance and the headings wanted are far apart. Folding the outline lets the page's top levels be seen at once and the details opened where they are needed.

## What Changes

- An outline entry that has headings under it (until the next heading of the same or a higher level) gets an arrow; clicking the arrow folds or unfolds the entries under it. Clicking the heading's text still scrolls to it.
- By default the outline is unfolded down to H3: H1 and H2 entries are unfolded, entries of H3 and below are folded, so H4-H6 under an H3 are hidden.
- What is folded or unfolded is kept per editor window while the page is edited (a heading keeps its state as long as its anchor stays the same).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: a new requirement for folding the outline, unfolded down to H3 by default.

## Impact

- The editor's outline (its view, a new message for the arrow, the fold state of the window), editor tests, README.
