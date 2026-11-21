# Proposal

## Why

After saving a map under a new name, Undo can restore its former filename. Pressing Ctrl+S then silently overwrites the original map.

## What Changes

- Keep the latest successful save destination when undoing or redoing edits, including edits made before the map was first saved.

## Capabilities

### Modified Capabilities

- `application-shell` — Save continues to write to the file chosen in Save As across undo and redo.

## Impact

The change affects map saving and its interaction with edit history.
