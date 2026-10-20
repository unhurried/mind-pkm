## ADDED Requirements

### Requirement: Drag to pan
Pressing the left mouse button on an empty part of the map and moving the pointer SHALL scroll the map with the pointer, both horizontally and vertically, until the button is released or the pointer leaves the window. The pointer SHALL show an open hand over empty map space and a closed hand while dragging. A press on a node, a link marker, a fold marker or a scrollbar SHALL keep its own behavior and SHALL NOT start a drag; the drag SHALL NOT change the selection or the map.

#### Scenario: Dragging the map
- **WHEN** the left button is pressed on empty map space and the pointer is moved 100 px to the left and 50 px up
- **THEN** the map content moves 100 px to the left and 50 px up with the pointer

#### Scenario: Press on a node
- **WHEN** the left button is pressed on a node and the pointer is moved
- **THEN** the node is selected and the map does not scroll

#### Scenario: Map edge
- **WHEN** the map is dragged beyond the end of its scrollable area
- **THEN** it stops at the edge
