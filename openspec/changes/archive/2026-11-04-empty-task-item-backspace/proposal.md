# Proposal

## Why

Deleting an empty task item with Backspace takes two presses that pass through a state nobody wants: the first takes only the box away and leaves an empty plain item (`- `), and only the second makes it an empty paragraph. The usual case is Enter after the last task of a list, which starts a new empty task item, and then Backspace to type something else below the list's item; the empty plain item in between is just one more key to press.

## What Changes

- Backspace at the start of an empty task item (`- [ ] `, `- [x] `, `1. [ ] `) removes its box and its bullet together: the item becomes the new empty paragraph it would become as an empty plain item (inside the item before it, or a paragraph of its own for the first item of a list at the top level).
- A task item with text keeps the current behavior: the first Backspace removes the box, the next one the bullet.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: new requirement Backspace in a task item (an empty one loses its box and bullet with one press); Blocks under a list item no longer states the box rule itself.

## Impact

- `src/markdown/edit.rs` (`backspace`), with tests there and in `src/app/editor/tests/lists.rs`.
- An empty task item can no longer be turned into an empty plain item with Backspace; typing `- ` in a new paragraph still makes one.
