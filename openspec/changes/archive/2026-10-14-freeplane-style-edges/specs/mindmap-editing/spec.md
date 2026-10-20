## MODIFIED Requirements

### Requirement: Right-only layout
The map SHALL be laid out with the root on the left and every subtree growing to the right of its parent, children stacked vertically and centered on their parent, joined to it by an edge. All children of the root SHALL be shown on the right, including nodes whose POSITION says left.

#### Scenario: Map made in Freeplane with left-side nodes
- **WHEN** a map whose root has children with `POSITION="left"` is opened
- **THEN** those children are shown on the right of the root like the others

## ADDED Requirements

### Requirement: Edges
Every node SHALL be underlined by a thin line, and each child SHALL be joined to its parent by a thin curved edge from the right end of the parent's underline to the left end of the child's. The edges SHALL turn steeply near the parent, so the edges to several children do not overlap.

#### Scenario: Several children
- **WHEN** a node has three children
- **THEN** three separate edges run from the right end of its underline, each to the underline of one child
