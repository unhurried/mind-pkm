# Proposal

## Why

After Ctrl+S the page editor's status bar showed `saved <full path of the page>`. With a long path it was drawn over the word and character counts, and the message stayed until another one replaced it, long after it stopped being relevant (issue #33). The map window had the same persistent message.

## What Changes

- Messages about saving and exporting name the file only, not its whole path.
- A status bar message never overlaps the rest of the status bar: it is cut off where its room ends.
- A message goes after about ten seconds; in the page editor, also with the next edit.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `application-shell`: status bar messages of the map window.
- `page-editor`: status bar messages of the editor windows.

## Impact

- `src/app/mod.rs` (a status message that expires), `src/app/view.rs`, `src/app/file.rs`, `src/app/editor/mod.rs`, `src/app/editor/view.rs`, `src/app/editor/find.rs`, `src/app/search_panel.rs`.
- Tests in `src/app/editor/tests.rs` and `src/app/tests.rs`.
