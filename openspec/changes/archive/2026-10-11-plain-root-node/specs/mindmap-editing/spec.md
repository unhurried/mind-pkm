## MODIFIED Requirements

### Requirement: Node decorations
A node SHALL show its icons before its text - the checkmark icon as a green check, with the text dimmed - and a link marker after it when it has a link. Every node, the root included, SHALL be drawn the same way: without a frame or background and with the same padding. Selected nodes SHALL be highlighted, the focused one with a stronger border.

#### Scenario: Checked node
- **WHEN** a node has Freeplane's `button_ok` icon
- **THEN** it is shown with a green check before its text and the text dimmed

#### Scenario: Root node
- **WHEN** the root node is not selected
- **THEN** it is shown without a frame or background, like any other unselected node
