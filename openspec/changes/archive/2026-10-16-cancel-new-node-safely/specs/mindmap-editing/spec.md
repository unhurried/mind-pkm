## MODIFIED Requirements

### Requirement: Structural editing
The application SHALL support: a new child node (Tab or Insert), a new sibling after the selected node (Enter), editing node text (Ctrl+E), deleting the selected nodes without touching files (Delete), moving a node up or down among its siblings (Ctrl+Up / Ctrl+Down), and setting or removing a node's link (Ctrl+K). A new node SHALL ask for its text at once; cancelling, or leaving the text empty, SHALL remove it again.

#### Scenario: Cancelling a new node
- **WHEN** Tab is pressed and the text dialog is cancelled
- **THEN** no node is added and the previously selected node is selected again

#### Scenario: Cancelling a new node while the map changes
- **WHEN** Tab is pressed, the Next Steps refresh changes the map while the text dialog is open, and the dialog is cancelled
- **THEN** the new node is gone and the refresh's changes stay, and Ctrl+Z returns to the map before the refresh without the new node
