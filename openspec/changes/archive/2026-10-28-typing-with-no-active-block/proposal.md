# Proposal

## Why

With no block active in the page editor (right after a page opens, or after a click that activates nothing), the first typed character was lost and the rest went into the source before the first block's marker (`bc# Page1`), turning a heading into a paragraph (issue #20). Home and End did nothing, so the keys after them took the same path.

## What Changes

- The first key typed with no block active starts editing at the cursor and is typed too.
- The cursor starts in what the block shows: in a heading, after its `#` marker, not before it.
- Home and End also start editing, at the start or end of the line.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: a new requirement on typing while no block is active.

## Impact

- `src/app/editor/mod.rs` (keys while nothing is active), `src/app/editor/page.rs` (a cursor placed in hidden source moves to where it shows).
- Tests in `src/app/editor/tests.rs`.
