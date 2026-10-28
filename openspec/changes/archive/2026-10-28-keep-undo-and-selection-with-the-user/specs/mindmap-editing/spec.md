## MODIFIED Requirements

### Requirement: Undo
Every change to the map, including changes made by PKM commands, SHALL be undoable with Ctrl+Z and redoable with Ctrl+Y (or Ctrl+Shift+Z), at least 100 steps back, and Undo and Redo SHALL select the nodes that were selected at that point. The Next Steps refresh SHALL NOT be an undo step: its changes SHALL be kept by Undo and Redo, which take back the user's own changes only. A command that changes nothing, such as New Page or New Directory that fail, SHALL add no undo step and SHALL NOT mark the map as changed. Changes to files in the document directory are not undone; renaming a page or directory with Edit (F2) therefore ends the undo history, so that Undo and Redo never bring back a map naming the files as they were.

#### Scenario: Undoing a refresh
- **WHEN** a node is pasted, the Next Steps refresh then changes the map, and Ctrl+Z is pressed
- **THEN** the paste is taken back, the refresh's changes stay, and the node selected before the paste is selected

#### Scenario: A command that fails
- **WHEN** F9 is pressed on a node whose text is not a valid page name
- **THEN** the error is shown, the map is not marked as changed, and Ctrl+Z does not take back anything older

#### Scenario: Undo after renaming a page
- **WHEN** a page node is renamed with F2 and Ctrl+Z is pressed
- **THEN** the node, its link and the ToDo items keep the new name, matching the renamed files

### Requirement: Selection and navigation
Clicking a node SHALL select it; Ctrl+click SHALL toggle it in the selection and Shift+click add it. The arrow keys SHALL move the selection: Right to the child nearest the node's vertical center, Left to the parent, Up/Down to the previous/next shown node of the same depth; Home SHALL select the root. The view SHALL scroll to keep the selected node visible, and Esc SHALL center it. When a filter or fold hides a selected node, the selection SHALL move to its nearest shown ancestor. When the selected node is deleted, by Delete, Delete (F11), Undo or the Next Steps refresh, the selection SHALL move to its next sibling, else its previous sibling, else its parent.

#### Scenario: Moving down past the last sibling
- **WHEN** the last child of a node is selected and Down is pressed
- **THEN** the selection moves to the first shown node of the same depth under the next parent (a cousin)

#### Scenario: Moving right into a folded node
- **WHEN** a folded node is selected and Right is pressed
- **THEN** the node unfolds and stays selected

#### Scenario: The selected node is deleted
- **WHEN** the selected directory node `B` between `A` and `C` is deleted with F11
- **THEN** `C` is selected

### Requirement: Structural editing
The application SHALL support: a new child node (Tab or Insert), a new sibling after the selected node (Enter), editing node text (Ctrl+E), deleting the selected nodes without touching files (Delete), moving a node up or down among its siblings (Ctrl+Up / Ctrl+Down), and setting or removing a node's link (Ctrl+K). A new node SHALL ask for its text at once; cancelling, or leaving the text empty, SHALL remove it again and leave the map's unsaved state as it was.

#### Scenario: Cancelling a new node
- **WHEN** Tab is pressed on a saved map and the text dialog is cancelled
- **THEN** no node is added, the previously selected node is selected again, and the map has no unsaved changes

#### Scenario: Cancelling a new node while the map changes
- **WHEN** Tab is pressed, the Next Steps refresh changes the map while the text dialog is open, and the dialog is cancelled
- **THEN** the new node is gone and the refresh's changes stay, and Ctrl+Z neither brings the new node back nor takes the refresh's changes away
