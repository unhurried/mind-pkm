# Proposal

## Why

A page's attachments live in its `<name>.assets/` folder, but the editor gives no view of them: to see what is there, open one, tidy it up or link to it, the user has to switch to Explorer and type the path into the page by hand.

## What Changes

- A pane on the right of each editor window lists the files and folders of the open file's `<name>.assets/` folder as a tree. View → Assets (Ctrl+Shift+2) shows and hides it; it is hidden when a window opens, and each window has its own.
- Double-clicking a file opens it (a `.md` file in the editor, any other file with its default application); double-clicking a folder expands or collapses it.
- Right-clicking an item offers Rename and Delete. Renaming rewrites the open page's links to the item as one undoable change; deleting asks first and moves the item to the Recycle Bin. Neither is offered for items outside the document directory.
- Dragging an item onto the page inserts a link to it where it is dropped: `![](…)` for an image, `[name](…)` for any other file or a folder.
- The pane is read again when it is shown, when the window is activated, after files are inserted into the page and after a rename or delete, and with its refresh button.

## Capabilities

### New Capabilities

- `page-assets`: the editor's assets pane - listing a page's assets folder, opening, renaming and deleting its items, and linking to them by drag and drop.

### Modified Capabilities

None.

## Impact

- New UI-free module for reading the assets tree, renaming (with the page's links) and deleting inside the document directory.
- `src/app/editor/`: the pane's state, view and messages, the `Assets` command (menu, Ctrl+Shift+2), the rename and delete dialogs, dragging from the pane onto rendered and active blocks.
- `src/app/`: the document directory and the Recycle Bin remover handed to editors.
- README: the editor shortcut table gains Ctrl+Shift+2.
