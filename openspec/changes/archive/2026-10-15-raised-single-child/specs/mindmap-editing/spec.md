## MODIFIED Requirements

### Requirement: Right-only layout
The map SHALL be laid out with the root on the left and every subtree growing to the right of its parent, children stacked vertically and centered on their parent, joined to it by an edge. A node with only one shown child SHALL instead have that child placed a little higher (8 px at 100% zoom, scaled with the zoom), so that parent and child do not read as one line. All children of the root SHALL be shown on the right, including nodes whose POSITION says left.

#### Scenario: Map made in Freeplane with left-side nodes
- **WHEN** a map whose root has children with `POSITION="left"` is opened
- **THEN** those children are shown on the right of the root like the others

#### Scenario: Single child
- **WHEN** a node has exactly one shown child of the same height
- **THEN** the child is shown slightly above the parent, and the edge between them bends up into the child's underline
