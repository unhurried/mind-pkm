# Proposal

## Why

Enter at the end of a list item whose nested items (or other blocks) follow after a blank line moved those nested items up a level: the new, empty item could not hold them, since an empty list item can't have a blank line before its first block. While the new item stayed empty, the items below showed one level too high.

## What Changes

- Enter at the end of a list item starts the new item with the item's nested items and other blocks below it, at their level. When a blank line separates them, the blank line goes before the new item instead.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: Enter in a list item with nested items.

## Impact

- `src/markdown/edit.rs` (Enter in a list item) and its tests.
