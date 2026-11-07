## ADDED Requirements

### Requirement: Outdenting a node
Ctrl+Left (Edit → Make Sibling of Parent) SHALL move the focused node, with its subtree, up one level: right after its parent, among the children of its grandparent. The move SHALL keep the nodes' IDs and SHALL be one undo step. For the root and for a child of the root nothing SHALL happen.

#### Scenario: Making a node a sibling of its parent
- **WHEN** the root has the children `a` and `c`, `a` has the children `a1`, `a2` and `a3`, and Ctrl+Left is pressed on `a2`
- **THEN** the root's children are `a`, `a2` and `c`, `a` has the children `a1` and `a3`, and Ctrl+Z restores them

#### Scenario: A child of the root
- **WHEN** Ctrl+Left is pressed on a child of the root, or on the root
- **THEN** nothing changes and the map has no unsaved changes
