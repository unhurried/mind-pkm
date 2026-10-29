## MODIFIED Requirements

### Requirement: Selection after deleting the selected node
When the selected node is deleted, by Delete (F11), Undo or the Next Steps refresh, the selection SHALL move to its next sibling, else its previous sibling, else its parent.

#### Scenario: The selected node is deleted
- **WHEN** the selected directory node `B` between `A` and `C` is deleted with F11
- **THEN** `C` is selected

### Requirement: Structural editing
The application SHALL support: a new child node (Tab or Insert), a new sibling after the selected node (Enter), editing node text (Ctrl+E), deleting the selected nodes (Delete, see Delete (F11)), moving a node up or down among its siblings (Ctrl+Up / Ctrl+Down), and setting or removing a node's link (Ctrl+K). A new node SHALL ask for its text at once; cancelling, or leaving the text empty, SHALL remove it again and leave the map's unsaved state as it was.

#### Scenario: Cancelling a new node
- **WHEN** Tab is pressed on a saved map and the text dialog is cancelled
- **THEN** no node is added, the previously selected node is selected again, and the map has no unsaved changes

#### Scenario: Cancelling a new node while the map changes
- **WHEN** Tab is pressed, the Next Steps refresh changes the map while the text dialog is open, and the dialog is cancelled
- **THEN** the new node is gone and the refresh's changes stay, and Ctrl+Z neither brings the new node back nor takes the refresh's changes away
