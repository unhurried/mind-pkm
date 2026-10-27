## ADDED Requirements

### Requirement: View while moving a node among its siblings
Moving a node up or down among its siblings SHALL NOT scroll the map while the moved node stays in view; when it would leave the view, the map SHALL scroll just enough to keep it in view.

#### Scenario: Moving a node within the view
- **WHEN** Ctrl+Down is pressed on a node whose next sibling is in view
- **THEN** the node takes its sibling's place and the map does not scroll

#### Scenario: Moving a node out of the view
- **WHEN** Ctrl+Down is pressed on a node at the bottom edge of the view
- **THEN** the map scrolls up just enough to keep the moved node in view
