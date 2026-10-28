## ADDED Requirements

### Requirement: Node text of several lines
The text dialog of a node (a new node, Ctrl+E, and Edit (F2) on a node that is neither a page nor a directory node) SHALL show the node's whole text, all its lines, with the cursor at the end, and SHALL let lines be added and removed: Enter SHALL confirm and Shift+Enter SHALL start a new line. The status bar SHALL show the selected node's path on one line, the lines of a node's text joined by spaces.

#### Scenario: Editing a node of three lines
- **WHEN** a node reads `line one`, `line two`, `line three` on three lines, Ctrl+E is pressed, `X` is typed and Enter pressed
- **THEN** the node reads `line one`, `line two`, `line threeX`

#### Scenario: Adding a line
- **WHEN** Shift+Enter is pressed at the end of a node's text in its text dialog, `next` is typed and Enter pressed
- **THEN** the node's text gets a last line `next`

#### Scenario: Path of a node of several lines
- **WHEN** the node `line one` / `line two` under the root is selected
- **THEN** the status bar shows `Root > line one line two` on one line
