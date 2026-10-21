## ADDED Requirements

### Requirement: Restoring the window
The application SHALL open its window with the size, position and maximized state it had when it last exited; without a remembered window it SHALL open at 1280×800. A window that was maximized SHALL open maximized and return to the size and position it had before being maximized when it is restored. The size and position of a minimized window SHALL NOT be remembered.

#### Scenario: Restart keeps the window
- **WHEN** the window is resized to 900×600, moved, and the application is closed and started again
- **THEN** the window opens at 900×600 at the same position

#### Scenario: Restart while maximized
- **WHEN** the application is closed while its window is maximized and started again
- **THEN** the window opens maximized, and restoring it returns to the size and position it had before being maximized

## MODIFIED Requirements

### Requirement: Per-user settings
The application SHALL keep its settings in `%APPDATA%\mind-pkm\`: the last opened map, the window's size, position and maximized state, how each map was last shown (zoom level, scroll position, selected nodes; the map file itself is not changed for it), and an optional `template.md` that replaces the built-in New Page template. A settings file that is not there SHALL be read from `%APPDATA%\freeplane-pkm\`, where versions under the former name kept it.

#### Scenario: Custom page template
- **WHEN** `%APPDATA%\mind-pkm\template.md` exists
- **THEN** New Page creates pages from it instead of the built-in template

#### Scenario: Settings from the former name
- **WHEN** only `%APPDATA%\freeplane-pkm\` holds the last opened map and a `template.md`
- **THEN** that map is opened on start and New Page uses that template
