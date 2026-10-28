## MODIFIED Requirements

### Requirement: Drag to pan
Pressing the left mouse button on an empty part of the map and moving the pointer SHALL scroll the map with the pointer, both horizontally and vertically, also past the edges of the map and when the whole map fits in the window, until the button is released or the pointer leaves the window. A press on a node, a link marker, a fold marker, a branch point or a scrollbar SHALL keep its own behavior and SHALL NOT start a drag; the drag SHALL NOT change the selection or the map.

#### Scenario: Dragging the map
- **WHEN** the left button is pressed on empty map space and the pointer is moved 100 px to the left and 50 px up
- **THEN** the map content moves 100 px to the left and 50 px up with the pointer

#### Scenario: Map edge
- **WHEN** the whole map fits in the window and empty map space is dragged 100 px to the right and 50 px down
- **THEN** the map moves 100 px to the right and 50 px down with the pointer, beyond the edge of the map area it had

#### Scenario: Press on a node
- **WHEN** the left button is pressed on a node and the pointer is moved
- **THEN** the node is selected and the map does not scroll

### Requirement: Map margin
The scrollable map area SHALL extend a narrow margin (40 px, independent of the zoom) beyond the map on every side, plus the room Esc and panning add where they need it to move the map past its edges; that room SHALL go again once it is scrolled out of view. Esc SHALL center the selected node wherever it is; centering a node when a map is opened, a node link is followed or a search result is selected SHALL scroll only as far as the map area allows.

#### Scenario: Scrolling to the end
- **WHEN** the map is larger than the window and is scrolled all the way down and to the right
- **THEN** the bottom-most and right-most nodes are 40 px from the bottom and right edges of the map area

#### Scenario: Centering the root
- **WHEN** a small map sits at the top left of the window and Esc is pressed with the root selected
- **THEN** the root moves to the center of the window
