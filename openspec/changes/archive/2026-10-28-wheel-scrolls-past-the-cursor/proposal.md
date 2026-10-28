# Proposal

## Why

Scrolling a page with the mouse wheel stopped as soon as the line with the cursor reached the top of the view (issue #30): the editor scrolled back to keep the cursor in view while the wheel scrolled away from it. The editor kept the cursor in view after every keyboard event no widget took, including key releases and modifier changes, whether or not the cursor had moved.

## What Changes

- The page scrolls to the cursor only when the cursor moved, the selection changed or the text was edited (or a menu command ran); the wheel and the scroll bar may take the cursor out of view.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: when the page follows the cursor.

## Impact

- `src/app/editor/mod.rs` (when the cursor is revealed).
- A test in `src/app/editor/tests.rs`.
