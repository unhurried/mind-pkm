## MODIFIED Requirements

### Requirement: Dialogs
Errors, confirmations and text input SHALL be shown as modal dialogs inside the window (for editor windows, see page-editor); several messages SHALL be shown one after another. Enter SHALL confirm and Esc SHALL cancel, also while a dialog's text box has the keyboard. The text of a node is no dialog input: it is edited in the node (see mindmap-editing, In-place text editing).

#### Scenario: Errors for several nodes
- **WHEN** New Page fails for two selected nodes
- **THEN** two error messages are shown one after the other
