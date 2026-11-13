## ADDED Requirements

### Requirement: Minimum map window height
The map window SHALL have a minimum height that leaves room for the search panel at its minimum height, the menu and status bars, and a usable map area. A remembered window height below this minimum SHALL be raised to the minimum when restored.

#### Scenario: Resizing a short window
- **WHEN** the map window is resized shorter with the search panel open
- **THEN** resizing stops while the panel controls and a map area remain visible

#### Scenario: Restoring an older small window
- **WHEN** a remembered map window height is below the minimum
- **THEN** the map window opens at the minimum height
