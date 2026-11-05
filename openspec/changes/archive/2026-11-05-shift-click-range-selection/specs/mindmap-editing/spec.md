## MODIFIED Requirements

### Requirement: Selection and navigation
Clicking a node SHALL select it; Ctrl+click SHALL toggle it in the selection. The arrow keys SHALL move the selection: Right to the child nearest the node's vertical center, Left to the parent, Up/Down to the previous/next shown node of the same depth; Home SHALL select the root. The view SHALL scroll to keep the selected node visible, and Esc SHALL center it. When a filter or fold hides a selected node, the selection SHALL move to its nearest shown ancestor.

#### Scenario: Moving down past the last sibling
- **WHEN** the last child of a node is selected and Down is pressed
- **THEN** the selection moves to the first shown node of the same depth under the next parent (a cousin)

#### Scenario: Moving right into a folded node
- **WHEN** a folded node is selected and Right is pressed
- **THEN** the node unfolds and stays selected

## ADDED Requirements

### Requirement: Range selection with Shift+click
Shift+click SHALL select the nodes from the anchor to the clicked node, both included, instead of the selection, and focus the clicked node: for siblings, the siblings between them (not their descendants); else every node shown between them in document order (a node before its children). Hidden nodes SHALL NOT be selected. The anchor SHALL be the node focused before the first of consecutive Shift+clicks.

#### Scenario: Shift+click on a sibling
- **WHEN** `A`, `B`, `C` and `D` are siblings, `B` has children, `A` is clicked and `D` is Shift+clicked
- **THEN** `A`, `B`, `C` and `D` are selected and the children of `B` are not

#### Scenario: Shift+click under another parent
- **WHEN** `P` has the children `A` and `B`, `Q` follows `P` with the child `C`, `A` is clicked and `C` is Shift+clicked
- **THEN** `A`, `B`, `Q` and `C` are selected

#### Scenario: Consecutive Shift+clicks
- **WHEN** `A`, `B`, `C` and `D` are siblings, `A` is clicked, `D` is Shift+clicked and then `B` is Shift+clicked
- **THEN** `A` and `B` are selected
