## ADDED Requirements

### Requirement: Map context menus
Right-clicking a node SHALL open a menu offering New Child Node, New Sibling Node, Edit Node Text, Cut, Copy, Paste, Fold / Unfold, Set Link and the PKM commands Open, Edit, Toggle Checkmark, Add to ToDo, New Page, New Directory, Delete and Open Externally. Right-clicking empty map space SHALL open a menu offering Paste, Center Selected Node, Zoom In, Zoom Out, Reset Zoom and Update Next Steps and Search Index.

#### Scenario: Right-click on empty space
- **WHEN** empty map space is right-clicked
- **THEN** a menu with Paste and the zoom commands opens and the selection is unchanged

#### Scenario: Running a command from the node menu
- **WHEN** a node is right-clicked and New Child Node is chosen
- **THEN** a child of that node is created as with Tab

### Requirement: Selection on right-click
A right-click on a node outside the selection SHALL select only that node before its menu opens; a right-click on a selected node SHALL keep the selection, so that the menu's commands act on every selected node.

#### Scenario: Right-click on an unselected node
- **WHEN** nodes A and B are selected and node C is right-clicked
- **THEN** only C is selected and its menu is open

#### Scenario: Right-click on a selected node
- **WHEN** nodes A and B are selected, B is right-clicked and Cut is chosen
- **THEN** both A and B are cut
