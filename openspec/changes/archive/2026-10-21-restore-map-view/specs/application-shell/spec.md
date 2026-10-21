## ADDED Requirements

### Requirement: Restoring the view
A map opened again SHALL be shown with the zoom level, scroll position and selected nodes it was last shown with, remembered when the application exits, when another map takes its place, and when the map is saved. Selected nodes that no longer exist SHALL be left out (the root is selected if none is left); a selected node hidden by the filter or a fold SHALL hand its selection to its nearest shown ancestor.

#### Scenario: Restart keeps the view
- **WHEN** the map is zoomed to 150%, scrolled, a node is selected, and the application is closed and started again
- **THEN** the map is shown at 150% at the same scroll position with the same node selected

#### Scenario: Selected node gone
- **WHEN** the node selected when the map was last shown no longer exists in the map file
- **THEN** the map opens with the root selected

## MODIFIED Requirements

### Requirement: Opening maps
On start the application SHALL open the map given as the first command-line argument (so `.mm` files can be opened with it from Explorer), else the last opened map if it still exists, else show a welcome screen with Open and New. File → Open (Ctrl+O) SHALL pick a `.mm` file; File → New (Ctrl+N) SHALL create a map with `config > docDirPath > <path>` and `ToDo` nodes. The map opens with the archive-hiding filter and the root centered, unless its last view is restored (see Restoring the view).

#### Scenario: Restart
- **WHEN** the application is closed with a map open and started again without arguments
- **THEN** the same map is opened

### Requirement: Per-user settings
The application SHALL keep its settings in `%APPDATA%\mind-pkm\`: the last opened map, how each map was last shown (zoom level, scroll position, selected nodes; the map file itself is not changed for it), and an optional `template.md` that replaces the built-in New Page template. A settings file that is not there SHALL be read from `%APPDATA%\freeplane-pkm\`, where versions under the former name kept it.

#### Scenario: Custom page template
- **WHEN** `%APPDATA%\mind-pkm\template.md` exists
- **THEN** New Page creates pages from it instead of the built-in template

#### Scenario: Settings from the former name
- **WHEN** only `%APPDATA%\freeplane-pkm\` holds the last opened map and a `template.md`
- **THEN** that map is opened on start and New Page uses that template
