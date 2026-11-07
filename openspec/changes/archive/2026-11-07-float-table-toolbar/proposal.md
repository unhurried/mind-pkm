## Why

When the cursor goes into a table, the editor shows the table's toolbar above it and the toolbar takes room in the page: the table and every block below it move down by the toolbar's height, right under the pointer that clicked a cell. Leaving the table moves them back. The jump makes the click land somewhere else than where the user looked, and is distracting every time a table is entered or left.

## What Changes

- The table toolbar floats: it is drawn over the page just above the table being edited, without taking room, so the table and the blocks around it stay where they are when the cursor goes into or leaves a table. It covers the bottom of the block above while it is shown, on an opaque background.
- When the top of the table is scrolled out of view while the cursor is in it, the toolbar stays at the top of the page's visible part (over the table) as long as some of the table is in view.
- The toolbar is hidden while a menu, a right-click menu or a dialog of the editor window is open, so it never covers them.
- The toolbar's commands (alignment, resize, delete table) stay as they are.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `table-editing`: a new requirement for where the table toolbar is shown - floating above the table without moving the table or the blocks around it, staying in view while the table is, hidden while a menu or dialog is open.

## Impact

- The page editor's view of the table being edited (the toolbar placed over the page instead of above the table in the column), a small widget that places an element over its content, editor and benchmark-harness tests, README (the toolbar's description).
