## ADDED Requirements

### Requirement: Zoom shortcuts
The map SHALL be zoomable in fixed steps from 50% to 300%, starting at 100% on every start and kept when another map is opened. Ctrl+Plus (also Ctrl+`=`, Ctrl+`;` and the keypad's plus) SHALL zoom in, Ctrl+Minus SHALL zoom out and Ctrl+0 SHALL reset the zoom to 100%. Zooming with the keyboard SHALL keep the selected node in place on screen and in view.

#### Scenario: Zooming in with the keyboard
- **WHEN** Ctrl+Plus is pressed at 100%
- **THEN** the map is shown at the next larger step, its text larger, and the selected node stays where it was on screen

#### Scenario: Limits
- **WHEN** Ctrl+Minus is pressed at 50%
- **THEN** the zoom stays at 50%

#### Scenario: Reset
- **WHEN** Ctrl+0 is pressed at 200%
- **THEN** the map is shown at 100%

### Requirement: Wheel zoom
Turning the mouse wheel with Ctrl held over the map SHALL zoom in (away from the user) or out (towards the user) one step per notch, keeping the map point under the pointer in place as far as the map's scrollable area allows, and SHALL NOT scroll the map.

#### Scenario: Zooming with the wheel
- **WHEN** the wheel is turned one notch away from the user with Ctrl held while the pointer is over a node
- **THEN** the map is shown one step larger, the same node is still under the pointer, and the map does not scroll

### Requirement: Zooming is a view change
Zooming SHALL scale the whole map - text, icons, markers, spacing and edges - and SHALL leave clicking, dragging nodes and panning working as before. It SHALL NOT change the map, its unsaved state or the undo history.

#### Scenario: Not a change of the map
- **WHEN** a saved map is zoomed
- **THEN** it has no unsaved changes and Undo does not undo the zoom

#### Scenario: Dragging at another zoom
- **WHEN** a node is dragged onto another node at 150%
- **THEN** it becomes that node's last child, as at 100%

## MODIFIED Requirements

### Requirement: Compact spacing
Nodes SHALL be packed closely: at 100% zoom, sibling subtrees 2 px apart and node boxes 1 px of vertical padding, about 25 px per single-line row.

#### Scenario: List of siblings
- **WHEN** a node has ten single-line children at 100% zoom
- **THEN** they take about 250 px of height
