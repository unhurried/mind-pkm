## MODIFIED Requirements

### Requirement: Saving and unsaved changes
The map SHALL be written only when saved: Ctrl+S (Save As for a new map) or Ctrl+Shift+S. The window title SHALL show the map name with `*` while there are unsaved changes. Creating or opening another map, or closing the window, with unsaved changes SHALL ask whether to save, discard or cancel, and SHALL go on once the map is saved, also after Save As for a new map.

Undo and Redo SHALL keep the map file chosen by the most recent successful Save As, including the first save of an untitled map.

#### Scenario: Closing with unsaved changes
- **WHEN** the window is closed with unsaved changes and Cancel is chosen
- **THEN** the application stays open with the changes

#### Scenario: Saving a new map before creating another
- **WHEN** a map that has never been saved has changes, New is chosen, Save is answered and a file is chosen in Save As
- **THEN** the map is written to that file and the new map is shown

#### Scenario: Save As cancelled
- **WHEN** a map that has never been saved has changes, the window is closed, Save is answered and Save As is cancelled
- **THEN** the application stays open with the changes

#### Scenario: Undo and save a copy
- **WHEN** a user makes two edits to `Original.mm`, saves it as `Copy.mm`, then presses Ctrl+Z and Ctrl+S
- **THEN** `Copy.mm` contains only the first edit and `Original.mm` retains its contents from before the edits

#### Scenario: Redo after the first save
- **WHEN** a user undoes an edit to an untitled map, saves it as `Plan.mm`, then presses Ctrl+Y and Ctrl+S
- **THEN** `Plan.mm` contains the redone edit and stays open under that name
