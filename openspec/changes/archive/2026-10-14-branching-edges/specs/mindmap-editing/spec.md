## MODIFIED Requirements

### Requirement: Edges
Every node SHALL be underlined by a thin line, and each child SHALL be joined by a thin edge from the right end of its parent's underline to the left end of its own. The edges to a node's children SHALL run as one line halfway to the children, then vertically, and turn with a rounded corner into each child's underline. Shared parts SHALL look like one thin line however many children there are.

#### Scenario: Several children
- **WHEN** a node has three children
- **THEN** one line runs from the right end of its underline, and branches near the children into three, each ending at the underline of one child

#### Scenario: Many children
- **WHEN** a node has twenty children
- **THEN** the line leaving the parent is as thin as an edge to a single child
