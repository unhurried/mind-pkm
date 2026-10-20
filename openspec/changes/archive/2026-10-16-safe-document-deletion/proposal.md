# Proposal

## Why

Delete (F11) removes files permanently, and a node whose text is empty while it links the document directory counts as a directory node, so confirming F11 on it deletes the whole document directory. Names that the file system resolves to something else (`..`, `aux`, a directory name ending with a dot) can be created or renamed to as well. Deleting is the one PKM command whose mistakes cannot be undone, so it needs to be safe by construction.

## What Changes

- A node is a page or directory node only when its page file or directory lies inside the document directory and is not the document directory itself; a node resolving there or outside it is reported as `not a page or directory`.
- Delete (F11) moves the files to the Recycle Bin instead of removing them permanently. A page or directory whose files cannot be moved there keeps its node and is reported (`failed to delete page: ...`, `failed to delete directory: ...`).
- Delete never removes the document directory itself or anything outside it, whatever the selected nodes' texts and links are.
- New Page, New Directory and Edit (F2) reject names that are empty or blank, `.` or `..`, contain control characters, or are Windows device names (`CON`, `AUX`, `NUL`, ...), and directory names ending with a dot or a space, with the existing "... includes invalid characters" messages.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `document-management`: which nodes are page/directory nodes, which names are accepted, and how Delete (F11) removes files (Recycle Bin, never outside the document directory).

## Impact

- `src/pkm/mod.rs`: name validation, page/directory node classification.
- `src/paths.rs`: a check that a path lies strictly inside a directory.
- `src/pkm/commands.rs`: New Page / New Directory / rename validation, Delete.
- `src/app/dialog.rs`: Delete moves the files to the Recycle Bin.
- New dependency: `trash` (moves files to the Windows Recycle Bin / the freedesktop trash).
- README: the F11 description.
