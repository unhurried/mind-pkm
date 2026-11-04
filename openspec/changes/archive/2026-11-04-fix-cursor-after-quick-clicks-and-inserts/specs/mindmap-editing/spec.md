## ADDED Requirements

### Requirement: Selection after cancelling a new node
Cancelling a new node, or leaving its text empty, SHALL select again the nodes that were selected before it was created, those still in the map, also when the map changed meanwhile (a refresh); when none of them is left, its parent SHALL be selected.

#### Scenario: Cancelling a new sibling
- **WHEN** the node `a` is selected, Enter is pressed and the new node's text is cancelled with Esc
- **THEN** no node is added and `a` is selected, not its parent

#### Scenario: Leaving a new sibling empty
- **WHEN** the node `a` is selected, Enter is pressed and Enter is pressed again without typing
- **THEN** no node is added and `a` is selected

#### Scenario: Cancelling a new sibling after a refresh
- **WHEN** the page node `Task` is selected, Enter is pressed, the Next Steps refresh changes the map while the new node is edited, and Esc is pressed
- **THEN** the refresh's changes stay and `Task` is selected
