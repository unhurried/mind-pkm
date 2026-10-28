# Proposal

## Why

While the cursor was in a Mermaid block, the preview below its source showed only `…`, however long one waited (issue #29). The diagram appeared only once the cursor left the block. The math preview worked.

## What Changes

- While a Mermaid block is being edited, its diagram (or the error) is shown below its source, and follows the source as it is typed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `markdown-rendering`: the diagram of a Mermaid block being edited.

## Impact

- `src/app/editor/view.rs` (the active block's preview), `src/app/editor/render.rs`.
- A test in `src/app/editor/tests.rs`.
