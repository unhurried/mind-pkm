## ADDED Requirements

### Requirement: View menu and zoom level
The View menu SHALL offer Zoom In (Ctrl+Plus), Zoom Out (Ctrl+Minus) and Reset Zoom (Ctrl+0), and the status bar SHALL show the zoom level.

#### Scenario: Zoom level
- **WHEN** the map is zoomed to 150%
- **THEN** the status bar shows 150%

## MODIFIED Requirements

### Requirement: Menus and shortcuts
A menu bar SHALL offer File, Edit, View and PKM menus listing every command with its shortcut. The PKM commands SHALL keep the add-on's function-key assignments (F1 Open, F2 Edit, F3 Fold One Level, F4 Unfold One Level, F5 Filter, F6 Search, F7 Toggle Checkmark, F8 Add to ToDo, F9 New Page, F10 New Directory, F11 Delete, F12 Update Next Steps and Search Index). While the search keyword box has the keyboard, only the function keys SHALL reach the map.

#### Scenario: Typing in the search box
- **WHEN** Tab or Delete is pressed while typing a search keyword
- **THEN** no node is created or deleted
