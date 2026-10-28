# Proposal

## Why

Edit (F2) on a page renames its files, but Ctrl+Z in the map went back to the map from before the rename: the node, its link and the ToDo items named the old page again while the files kept the new name. The next refresh then found the page unreadable and deleted its ToDo items, including ones added by hand. In an open editor of the page, Ctrl+Z could likewise undo the rewrite of the `.assets` references that the rename made inside the page, breaking the assets link (issue #23).

## What Changes

- A page or directory rename is the end of the map's undo history: Undo and Redo do not go back across it, so the map never names files as they were before the rename.
- In an open editor, a rename that rewrote the page's text ends that editor's undo history too; a rename that left the text alone (a directory) keeps it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: Undo does not cross a rename of a page or directory.
- `page-editor`: an open page's undo history does not cross the rewrite a rename made.

## Impact

- `src/app/doc.rs`, `src/app/dialog.rs`: a successful rename drops the map's undo and redo steps.
- `src/app/editor/page.rs`, `src/markdown/history.rs`: following a rename that changed the text clears the page's history.
- Tests in `src/app/editor/tests.rs`.
