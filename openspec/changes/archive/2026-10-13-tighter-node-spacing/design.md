# Design

## Decisions

- **Line height, not padding**: the padding (1 px) and the gap were already small; most of a row's height was the default line height (1.3). 1.2 still leaves room for Japanese glyphs and descenders.
- **Gap of 1 px, not 0**: keeps the borders of two selected neighbors from touching.
- **Icons follow the text height**: icons are glyphs drawn at the node font size with the same line height, so the former minimum height of an icon (18 px) is dropped.
