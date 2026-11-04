## MODIFIED Requirements

### Requirement: Edit (F2) on other nodes
Edit on a node that is neither a page nor a directory node SHALL edit its text in the node on the map (see mindmap-editing, In-place text editing); double-clicking a node SHALL do the same as Edit.

#### Scenario: Plain node
- **WHEN** F2 is pressed on a node without a link and a new text is entered in the node
- **THEN** only the node's text changes

#### Scenario: Double-clicking a page node
- **WHEN** a node that links a page is double-clicked
- **THEN** the dialog asking for a new name is shown, as with F2
