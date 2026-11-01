# Proposal

## Why

The page view draws headings with a rule, a band and chips, but exported HTML and PDF still used the old style sheet's headings (sizes alone, a thin rule below levels 1 and 2), so an exported page looked different from the page it came from.

## What Changes

- The built-in export style sheet draws headings as the page view does: the same sizes (2, 1.5, 1.25, 1.125, 1 and 1 times the body size), a 3px rule below level 1, a light band with an 8px chip for level 2, an 8px chip and a 1px rule for level 3, a 4px muted chip for level 4, levels 5 and 6 plain (level 6 muted).
- The band and the chips are printed in PDF export too (their colors are kept when printing), and a heading is kept on the page of the text that follows it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-export`: new requirement for the design of exported headings.

## Impact

- `assets/export.css` (the built-in style sheet). A custom `export.css` is unaffected.
