## Why

Reordering a list or a task list in the page editor means cutting an item (with its nested items) and pasting it elsewhere. The map already moves a node among its siblings with Ctrl+Up / Ctrl+Down; the page editor should move a list item the same way.

## What Changes

- Ctrl+Up / Ctrl+Down in a list item (bullet, ordered or task item, at any depth, also inside quotes and list items) swap it, with its nested items and the blocks under it, with the previous / next item of the same list. The cursor stays in the moved item.
- An ordered list keeps its numbers in their places: the moved items take each other's numbers.
- At the first / last item of its list, and outside list items, the keys do nothing.
- New Paragraph menu entries Move Item Up / Move Item Down with these shortcuts; they work in the source mode too.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: moving list items.

## Impact

- `src/markdown/edit.rs` (the rule), `src/app/editor/keys.rs` (commands, menu, binding), `src/app/editor/mod.rs` (running the commands), editor tests, README's shortcut table.
