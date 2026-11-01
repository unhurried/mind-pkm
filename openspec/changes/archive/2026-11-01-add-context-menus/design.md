# Design

## Context

Both windows already draw drop-down menus from a shared function (`ui::menu_dropdown`), and the editor has two right-click menus of its own: the table cell's (row and column commands) and the assets pane item's (Rename, Delete). The map has no right-click handling at all; the page's text handles only the left button.

## Decisions

- **One context menu widget for every window.** `ui::context_menu` draws a list of items and separators at a point of the window, moved inside the window when it would not fit, with the same look and shortcut column as the menu bar's drop-downs; a press beside it closes it. The map menus, the page menu and the table cell's menu use it.
- **Where the menu opens.** A thin wrapper at the window's root reports where the right button went down (window coordinates) before the widgets under the pointer handle the press; those widgets then say which menu opens (a node, empty map space, the page, a table cell). This keeps the menu at the pointer without tracking every pointer move.
- **Items are constant lists next to the menu bar's.** Their labels and shortcuts are those of the menu bar items running the same commands (a test checks this), so the two never disagree.
- **Every item is enabled.** A command that does not apply behaves as from the menu bar or its shortcut (same messages); greying items out would need each command's preconditions duplicated in the view.
- **Map selection.** As in a file manager: a right-click on a node outside the selection selects only that node; on a selected node the selection (several nodes) stays, so the menu acts on all of them. A right-click on empty space keeps the selection.
- **Page cursor.** A right-click inside the selection keeps it, so Cut/Copy/format act on it. Anywhere else the press is handled like a left click (the same position rules, the block becomes active), only without starting a drag, and then the menu opens.
- **Esc** closes an open context menu first, before anything else it does in the window.

## Risks / Trade-offs

- The map's PKM commands in the node menu show their messages when the node is not a page or directory node, rather than being greyed out.
