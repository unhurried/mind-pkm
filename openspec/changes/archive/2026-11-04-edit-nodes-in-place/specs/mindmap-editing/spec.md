## MODIFIED Requirements

### Requirement: Structural editing
The application SHALL support: a new child node (Tab or Insert), a new sibling after the selected node (Enter), editing node text (Ctrl+E), deleting the selected nodes (Delete, see Delete (F11)), moving a node up or down among its siblings (Ctrl+Up / Ctrl+Down), and setting or removing a node's link (Ctrl+K). A new node SHALL ask for its text at once, and a node's text SHALL be edited, in the node itself on the map (see In-place text editing); cancelling, or leaving the text empty, SHALL remove a new node again and leave the map's unsaved state as it was.

#### Scenario: Cancelling a new node
- **WHEN** Tab is pressed on a saved map and Esc is pressed in the new node
- **THEN** no node is added, the previously selected node is selected again, and the map has no unsaved changes

#### Scenario: Cancelling a new node while the map changes
- **WHEN** Tab is pressed, the Next Steps refresh changes the map while the new node is edited, and Esc is pressed
- **THEN** the new node is gone and the refresh's changes stay, and Ctrl+Z neither brings the new node back nor takes the refresh's changes away

## ADDED Requirements

### Requirement: In-place text editing
The text of a new node, and the text of a node edited with Edit Node Text (Ctrl+E), Edit (F2) on a node that is neither a page nor a directory node, or a double-click, SHALL be edited in the node itself on the map: the node shows a text box with the node's font, size and wrapping, holding the whole text selected for an existing node. The node SHALL grow and shrink with the text, and the other nodes move to make room.

#### Scenario: Editing a node's text
- **WHEN** Ctrl+E is pressed on node `a`, `b` is typed and Enter is pressed
- **THEN** the node's text is `b`, there is no dialog and Ctrl+Z restores `a`

#### Scenario: A long text wraps
- **WHEN** a text wider than 400 px at 100% zoom is typed into a node
- **THEN** the node wraps the text into lines no wider than 400 px while it is typed


### Requirement: Confirming and cancelling the text
Enter or a click elsewhere on the window SHALL confirm the text being edited in a node, and Esc SHALL cancel it. Confirming a changed text SHALL be one undo step.

#### Scenario: Cancelling an edit
- **WHEN** Ctrl+E is pressed on node `a`, `b` is typed and Esc is pressed
- **THEN** the node's text is still `a` and the map has no unsaved changes

#### Scenario: Leaving a new node's text empty
- **WHEN** Tab is pressed and Enter is pressed without typing
- **THEN** no node is added and the map has no unsaved changes

#### Scenario: Confirming by a click
- **WHEN** a node's text is edited and another node is clicked
- **THEN** the text is confirmed and the clicked node is selected

### Requirement: One line, nothing else acting
The text edited in a node SHALL stay on one line: a line break cannot be entered, and line breaks of pasted text become spaces. While a node's text is edited, the map's keyboard shortcuts SHALL NOT act, and the node SHALL be shown even when the filter hides it.

#### Scenario: A shortcut while editing
- **WHEN** a node's text is edited and Delete is pressed
- **THEN** no node is deleted

#### Scenario: Pasting line breaks
- **WHEN** the text `a\nb` is pasted into a node being edited
- **THEN** the node's text holds `a b`

#### Scenario: A new node under a filter
- **WHEN** a filter is set that the new node's empty text does not match and Tab is pressed
- **THEN** the new node is shown and edited all the same
