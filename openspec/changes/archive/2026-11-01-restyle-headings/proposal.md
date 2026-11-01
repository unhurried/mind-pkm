# Proposal

## Why

Headings were told apart by size alone (levels 1 and 2 with a thin rule). A monochrome design with a rule, a band and chips marks the levels more clearly and follows the colors of the theme.

## What Changes

- Headings of levels 1-4 are decorated: a thick rule below level 1, a light band with a chip at its left edge for level 2, a chip and a thin rule for level 3, a small muted chip for level 4.
- The sizes of the headings are 2, 1.5, 1.25, 1.125, 1 and 1 times the body size; level 6 is shown in the muted color.
- The decoration is the same for a heading shown rendered and for the heading being edited, so the text does not move when the cursor enters it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `markdown-rendering`: new requirement for the design of headings.

## Impact

- `src/app/editor/view/` (new `heading.rs`, sizes in `theme.rs`, `blocks.rs` and `active.rs` put a heading in its frame).
