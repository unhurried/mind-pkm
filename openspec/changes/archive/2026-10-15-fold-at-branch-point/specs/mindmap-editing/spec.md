## MODIFIED Requirements

### Requirement: Folding
A folded node SHALL hide its subtree and show the number of its children next to it; clicking that number, or pressing Space with the node selected, SHALL toggle the fold. Clicking the point where the edges from a node other than the root to its children branch SHALL fold the node; hovering that point SHALL show a marker there. The folded state SHALL be stored in the map (FOLDED attribute).

#### Scenario: Unfolding by click
- **WHEN** the child count next to a folded node is clicked
- **THEN** the node unfolds and its children are shown

#### Scenario: Folding by clicking the branch point
- **WHEN** the branch point of the edges between an unfolded node and its children is clicked
- **THEN** the node folds, its children are hidden and the child count is shown next to it

#### Scenario: Undoing a fold by click
- **WHEN** a node is folded by clicking its branch point and Ctrl+Z is pressed
- **THEN** the node is unfolded again

### Requirement: Drag to pan
Pressing the left mouse button on an empty part of the map and moving the pointer SHALL scroll the map with the pointer, both horizontally and vertically, until the button is released or the pointer leaves the window. A press on a node, a link marker, a fold marker, a branch point or a scrollbar SHALL keep its own behavior and SHALL NOT start a drag; the drag SHALL NOT change the selection or the map.

#### Scenario: Dragging the map
- **WHEN** the left button is pressed on empty map space and the pointer is moved 100 px to the left and 50 px up
- **THEN** the map content moves 100 px to the left and 50 px up with the pointer

#### Scenario: Press on a node
- **WHEN** the left button is pressed on a node and the pointer is moved
- **THEN** the node is selected and the map does not scroll

#### Scenario: Map edge
- **WHEN** the map is dragged beyond the end of its scrollable area
- **THEN** it stops at the edge
