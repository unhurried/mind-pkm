## MODIFIED Requirements

### Requirement: Opening maps
On start the application SHALL open the map given as the first command-line argument (so `.mm` files can be opened with it from Explorer), else the last opened map if it still exists, else show a welcome screen with Open and New. File → Open SHALL pick a `.mm` file; File → New SHALL create a map with `config > docDirPath > <path>` and `ToDo` nodes. The map opens with the archive-hiding filter and the root centered, unless its last view is restored (see Restoring the view).

#### Scenario: Restart
- **WHEN** the application is closed with a map open and started again without arguments
- **THEN** the same map is opened

### Requirement: Menus and shortcuts
A menu bar SHALL offer File, Edit, View and PKM menus listing every command with its shortcut. The PKM commands SHALL keep the add-on's function-key assignments (F1 Open, F2 Edit, F3 Fold One Level, F4 Unfold One Level, F5 Filter, F6 Search, F7 Toggle Checkmark, F8 Add to ToDo, F9 New Page, F10 New Directory, F11 Delete, F12 Update Next Steps and Search Index) and SHALL also be reachable by Ctrl+O (Open), Ctrl+Shift+F (Filter), Ctrl+F (Search), Ctrl+N (New Page) and Ctrl+Shift+N (New Directory). File → New and File → Open SHALL have no shortcut. While the search keyword box has the keyboard, only the function keys and Ctrl+F SHALL reach the map.

#### Scenario: Typing in the search box
- **WHEN** Tab or Delete is pressed while typing a search keyword
- **THEN** no node is created or deleted

#### Scenario: Ctrl+N creates a page
- **WHEN** Ctrl+N is pressed with a node selected
- **THEN** the New Page command runs as with F9
