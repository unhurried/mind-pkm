## ADDED Requirements

### Requirement: Map margin
The scrollable map area SHALL extend only a narrow margin (40 px, independent of the zoom) beyond the map on every side. Centering a node SHALL scroll only as far as the map area allows.

#### Scenario: Scrolling to the end
- **WHEN** the map is larger than the window and is scrolled all the way down and to the right
- **THEN** the bottom-most and right-most nodes are 40 px from the bottom and right edges of the map area

#### Scenario: Centering the root
- **WHEN** Esc is pressed with the root selected
- **THEN** the root is shown 40 px from the left edge of the map area, vertically as near the middle as scrolling allows
