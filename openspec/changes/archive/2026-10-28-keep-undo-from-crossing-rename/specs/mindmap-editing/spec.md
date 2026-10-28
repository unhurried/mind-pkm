## MODIFIED Requirements

### Requirement: Undo
Every change to the map, including changes made by PKM commands and the background Next Steps refresh, SHALL be undoable with Ctrl+Z and redoable with Ctrl+Y (or Ctrl+Shift+Z), at least 100 steps back. Changes to files in the document directory are not undone; renaming a page or directory with Edit (F2) therefore ends the undo history, so that Undo and Redo never bring back a map naming the files as they were.

#### Scenario: Undoing a refresh
- **WHEN** the Next Steps refresh changed the map and Ctrl+Z is pressed
- **THEN** the map returns to its state before the refresh

#### Scenario: Undo after renaming a page
- **WHEN** a page node is renamed with F2 and Ctrl+Z is pressed
- **THEN** the node, its link and the ToDo items keep the new name, matching the renamed files
