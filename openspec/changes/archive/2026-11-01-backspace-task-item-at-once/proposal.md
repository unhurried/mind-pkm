# Proposal

## Why

Backspace at the start of a task item's text took off only its box, leaving an empty or plain list item, and a second Backspace was needed to take off the bullet. Deleting a task item that way showed a list item nobody asked for.

## What Changes

- Backspace at the start of a task item's text takes off its box and its bullet together, the same as for a plain list item: the item becomes a block of the item before it (or of the item its list is nested in), or a paragraph when it is the first item of a list at the top level.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: the requirement on blocks under a list item no longer takes a task item's box off first.

## Impact

- `src/markdown/edit.rs` (`backspace`).
