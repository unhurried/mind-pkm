## ADDED Requirements

### Requirement: Context menus
A right-click menu, in the map window or an editor window, SHALL open at the pointer, moved inside the window when it would not fit. Each item SHALL show the shortcut of its command, if it has one, and run the command as the menu bar does. The menu SHALL close when an item is chosen, when the pointer is pressed beside it, or with Esc, which then does nothing else.

#### Scenario: Near the window's edge
- **WHEN** a node near the bottom right corner of the window is right-clicked
- **THEN** the menu opens above and to the left of the pointer, entirely inside the window

#### Scenario: Closing with Esc
- **WHEN** a context menu of the map is open and Esc is pressed
- **THEN** the menu closes and the view is not centered on the selected node

#### Scenario: Pressing beside the menu
- **WHEN** a context menu is open and the pointer is pressed outside it
- **THEN** the menu closes and no command runs
