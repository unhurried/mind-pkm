# Proposal

## Why

Clicking some empty space in the page editor put the cursor nowhere: the caret disappeared and what was typed next was dropped without a sign (issue #28). It happened in the gap between two blocks, to the right of an empty list item (the template's `* ` under `### Next Steps`), and to the right of a line with inline math.

## What Changes

- A click in the gap between two blocks puts the cursor into the nearer block: at the end of the one above (upper half) or at the start of the one below (lower half).
- A click anywhere in the row of an empty list item puts the cursor into that item, not only on its bullet.
- A click to the right of a line with math, an image or another object puts the cursor at the end of that line, as in a plain line.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: clicking beside a block's content or in the gap between blocks.

## Impact

- `src/app/editor/view.rs` (the page column).
- Tests in `src/app/editor/tests.rs`.
