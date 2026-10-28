# Proposal

## Why

Closing the window with an unsaved map and unsaved pages asks about the map first. Answering Don't Save marked the map as saved at once, so cancelling the following question about the pages left the application open with a map that no longer showed `*`. The next close then ended the application without asking, and the map's changes were lost (issue #22).

## What Changes

- Don't Save no longer marks the map as saved. Cancelling the pages question aborts the whole exit: the map keeps its unsaved changes (`*`) and the next close asks about it again.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `application-shell`: a scenario for cancelling the pages question after discarding the map's changes.

## Impact

- `src/app/dialog.rs`: discarding no longer clears the map's unsaved flag (a new or opened map replaces it anyway).
- Test in `src/app/editor/tests.rs`.
