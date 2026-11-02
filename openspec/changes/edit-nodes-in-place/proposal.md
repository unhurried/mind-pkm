# Proposal

## Why

Editing a node's text, or typing the text of a new node, opens a modal dialog. The text is then entered away from the map, without the node's font, width and place. Editing the text where the node is shown is the usual way mind maps are edited, and keeps the map in view while typing.

## What Changes

- Editing a node's text (Ctrl+E, F2 on a node that is neither a page nor a directory node, double-click, Edit Node Text in the menus) and the text of a new node (Tab, Insert, Enter) are entered in the node itself on the map, not in a dialog.
- While the text is edited the node grows and shrinks with the text, and the nodes around it move; Enter or a click elsewhere confirms, Esc cancels. Cancelling, or leaving the text of a new node empty, removes the new node as before.
- A line break cannot be entered; pasted line breaks become spaces.
- Set Link, Filter and renaming a page or directory with F2 keep their dialogs.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: Structural editing (the new node's and the edited text's entry).
- `document-management`: Edit (F2) on other nodes.
- `application-shell`: Dialogs (text input in dialogs no longer covers node text).

## Impact

- `src/app/dialog.rs` (node text is no dialog input any more), `src/app/mod.rs`, `command.rs`, `keys.rs`, `doc.rs` (the layout takes the text being edited), `view.rs` (the node being edited shows a text editor), `tests.rs`.
