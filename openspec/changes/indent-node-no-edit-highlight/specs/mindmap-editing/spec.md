## MODIFIED Requirements

### Requirement: Node decorations
A node SHALL show its icons before its text - the checkmark icon as a green check, with the text dimmed - and a link marker after it when it has a link. Every node, the root included, SHALL be drawn the same way: without a frame or background and with the same padding. Selected nodes SHALL be highlighted, the focused one with a stronger border; the node whose text is being edited SHALL have no background.

#### Scenario: Checked node
- **WHEN** a node has Freeplane's `button_ok` icon
- **THEN** it is shown with a green check before its text and the text dimmed

#### Scenario: Root node
- **WHEN** the root node is not selected
- **THEN** it is shown without a frame or background, like any other unselected node

#### Scenario: Node being edited
- **WHEN** the text of the selected node is edited on the map
- **THEN** the node is drawn with its border but without the background of a selected node

## ADDED Requirements

### Requirement: Indenting a node
Ctrl+Right (Edit → Make Child of Node Above) SHALL move the focused node, with its subtree, to the end of the children of the node above it, its previous sibling, and unfold that node. The move SHALL keep the nodes' IDs and SHALL be one undo step. For the root and for a first child nothing SHALL happen.

#### Scenario: Making a node a child of the node above
- **WHEN** the root has the children `a`, `b` and `c` and Ctrl+Right is pressed on `b`
- **THEN** `b` is the last child of `a`, the root's children are `a` and `c`, and Ctrl+Z restores them

#### Scenario: A first child
- **WHEN** Ctrl+Right is pressed on the first child of a node
- **THEN** nothing changes and the map has no unsaved changes
