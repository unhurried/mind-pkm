## ADDED Requirements

### Requirement: Drag and drop nodes
Pressing the left mouse button on a node and dragging it onto another node SHALL move the node: dropped on the top quarter of the target it SHALL become the target's previous sibling, on the bottom quarter its next sibling, and elsewhere on the target its last child (the target is unfolded). The root SHALL only take children and SHALL NOT be dragged. A move SHALL keep the nodes' IDs and SHALL be one undo step.

#### Scenario: Drop as a child
- **WHEN** node `a` is dragged onto the middle of node `b`
- **THEN** `a` with its subtree becomes the last child of `b`

#### Scenario: Drop as a sibling
- **WHEN** node `a` is dragged onto the top edge of node `c`
- **THEN** `a` is placed right before `c` under `c`'s parent

#### Scenario: Drop onto a descendant
- **WHEN** node `a` is dragged onto one of its own children
- **THEN** the map does not change

#### Scenario: Undo
- **WHEN** a node was moved by drag and drop and Ctrl+Z is pressed
- **THEN** the node is back at its previous place

### Requirement: Dragging several nodes
A press without modifiers on a node of a multiple selection SHALL keep the selection; dragging it then SHALL move all selected nodes together, keeping their order in the map, and releasing it without dragging SHALL select just that node.

#### Scenario: Two selected nodes
- **WHEN** nodes `a` and `b` are selected and `b` is dragged onto the middle of node `c`
- **THEN** `a` and `b` become the last children of `c`, `a` before `b`

#### Scenario: Click in a multiple selection
- **WHEN** nodes `a` and `b` are selected and `b` is clicked without moving the pointer
- **THEN** only `b` is selected

### Requirement: Drag and drop feedback and cancelling
While nodes are dragged, the drop position SHALL be shown: an outline around the target for a child, a line along its top or bottom edge for a sibling. A drop onto a dragged node or one of its descendants, or not onto a node, SHALL change nothing; Esc or the pointer leaving the window SHALL cancel the drag. A press without moving the pointer SHALL only select the node.

#### Scenario: Cancelling with Esc
- **WHEN** a node is being dragged and Esc is pressed before the button is released
- **THEN** the map does not change

#### Scenario: Click without moving
- **WHEN** a node is pressed and released without moving the pointer
- **THEN** the node is selected and the map does not change
