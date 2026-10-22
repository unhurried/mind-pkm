## MODIFIED Requirements

### Requirement: Saving and unsaved changes
The map SHALL be written only when saved: Ctrl+S (Save As for a new map) or Ctrl+Shift+S. The window title SHALL show the map name with `*` while there are unsaved changes. Creating or opening another map, or closing the window, with unsaved changes SHALL ask whether to save, discard or cancel, and SHALL go on once the map is saved, also after Save As for a new map.

#### Scenario: Closing with unsaved changes
- **WHEN** the window is closed with unsaved changes and Cancel is chosen
- **THEN** the application stays open with the changes

#### Scenario: Saving a new map before creating another
- **WHEN** a map that has never been saved has changes, New is chosen, Save is answered and a file is chosen in Save As
- **THEN** the map is written to that file and the new map is shown

#### Scenario: Save As cancelled
- **WHEN** a map that has never been saved has changes, the window is closed, Save is answered and Save As is cancelled
- **THEN** the application stays open with the changes
