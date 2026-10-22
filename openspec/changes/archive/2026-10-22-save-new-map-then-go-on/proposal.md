# Proposal

## Why

Answering Save to the unsaved changes question for a map that has never been saved opens Save As. After saving, closing the window goes on and closes it, but creating or opening another map stops there: the user has to choose New or Open again. Exiting was handled by a variant of Save As of its own; New and Open never were.

## What Changes

- Whatever the unsaved changes question was asked for - a new map, opening another map, closing the window - goes on once the map is saved, also when saving needed Save As first.
- Cancelling Save As, or a failed save, still keeps the map open with its changes.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `application-shell`: Saving from the unsaved changes question goes on with what it was asked for once saved.

## Impact

- `src/app/file.rs`: Save As carries what to do once saved (replacing the Save-As-then-exit variant); saving goes on with it.
- `src/app/dialog.rs`: the Save answer just saves with what the question was asked for.
