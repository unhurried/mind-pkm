# Proposal

## Why

Opening a long page, zooming the editor or changing the width of its page laid out every block of the page at once, to know how tall each one is: about 0.25 s for a page of 100 KB and 1 s for one of 400 KB at every zoom step, and up to 0.2 s for every resize event while the window's edge is dragged. To answer at once, the editor lays out only the blocks near the visible part and measures the others a few at a time afterwards, taking a guess at their heights meanwhile. As the guesses above the visible part turn into heights, the text in view would move up or down - and on zooming, the text at the top of the page would move away, as the scroll position stays where it was in pixels.

## What Changes

- When blocks above the visible part change height without being edited - the page zoomed, the window resized, math or a diagram rendered, a block's height measured - the page scrolls with them, so that the block at the top of the visible part stays where it is (on zooming or resizing, as far into it as before for its new height).
- Opening a page, zooming and resizing lay out the blocks near the visible part only; the others are measured shortly after, a few at a time, while the page can be used.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: new requirement Position kept as the page is laid out anew.

## Impact

- `src/app/editor/near.rs` (guessed heights, measuring a few blocks at a time, where the block at the top moved), `src/app/editor/view/document.rs` (where the blocks were laid out), `src/app/keyed_column.rs` (tells where its children are), `src/app/scroll_watch.rs` (scrolls the content right after it is laid out), `src/app/after_draw.rs` (a message once the window is drawn, while blocks are left to measure); tests in `src/app/perf.rs`.
- No new dependencies; the `.mm` reader/writer and the document directory are untouched.
