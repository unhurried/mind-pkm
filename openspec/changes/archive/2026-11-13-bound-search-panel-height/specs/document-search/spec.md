## ADDED Requirements

### Requirement: Search panel height bounds
The search panel SHALL follow the map window's height between a minimum showing approximately four result rows and a maximum showing approximately ten result rows, with the keyword controls, headings and footer visible at both bounds.

#### Scenario: Short window
- **WHEN** the map window is reduced to its minimum height with search open
- **THEN** the panel has room for approximately four result rows and its controls remain visible

#### Scenario: Tall window
- **WHEN** the map window is made taller after the search panel reaches its maximum height
- **THEN** the panel stays at a height showing approximately ten result rows and the extra height goes to the map
