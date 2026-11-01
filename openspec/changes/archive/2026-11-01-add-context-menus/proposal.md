# Proposal

## Why

Every command of the map and of the page editor is reachable only from the menu bar or a shortcut. A right-click menu puts the commands that apply to what is under the pointer next to it, as in other desktop applications.

## What Changes

- Right-clicking a node in the map opens a menu with the node commands (new child/sibling, edit text, cut/copy/paste, fold, set link) and the PKM commands that act on nodes. A node outside the selection becomes the only selected node first; a right-click on a selected node keeps the selection.
- Right-clicking empty map space opens a menu with Paste, Center Selected Node, the zoom commands and Update Next Steps and Search Index.
- Right-clicking the page in an editor window opens a menu with the clipboard commands, Select All and the inline formats. A right-click outside the selection first puts the cursor where a click would; one inside the selection keeps it.
- The table cell's right-click menu starts with Cut, Copy, Paste and Select All, followed by its row and column commands.
- Every context menu opens at the pointer (moved inside the window when it would not fit), shows each item's shortcut, and closes when an item is chosen, on a click beside it, or with Esc.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `application-shell`: new requirement for how context menus open and close.
- `mindmap-editing`: new requirements for the map's right-click menus and the selection on right-click.
- `page-editor`: new requirements for the page's right-click menu and the cursor on right-click.
- `table-editing`: new requirement for the clipboard commands of the cell's right-click menu.

## Impact

- `src/app/ui.rs` (shared context menu), a widget telling where the right button went down, `src/app/view.rs` / `mod.rs` (map menus), `src/app/editor/` (page menu, cell menu, the text widgets' right-click).
- README, version 0.15.0.
