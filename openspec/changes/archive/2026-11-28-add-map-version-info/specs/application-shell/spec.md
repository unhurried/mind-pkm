# Spec Delta

## ADDED Requirements

### Requirement: Application version information
The map window SHALL offer About Mind PKM... in its Help menu, including when no map is open. Choosing the command SHALL show a modal dialog with the application name and the version of the running build. OK, Enter and Esc SHALL dismiss the dialog.

#### Scenario: Viewing the application version
- **WHEN** the user chooses Help → About Mind PKM... with a map open
- **THEN** the dialog shows Mind PKM and the running version
- **AND** dismissing the dialog returns to the same map

#### Scenario: Viewing the version from the welcome screen
- **WHEN** the user chooses Help → About Mind PKM... on the welcome screen
- **THEN** the dialog shows Mind PKM and the running version

## MODIFIED Requirements

### Requirement: Menus and shortcuts
The map window's menu bar SHALL offer File, Edit, View, PKM and Help menus listing every command with its shortcut when one is assigned; in every window, the menu names SHALL be evenly spaced. The PKM commands SHALL keep the add-on's function-key assignments (F1 Open, F2 Edit, F3 Fold One Level, F4 Unfold One Level, F5 Filter, F6 Search, F7 Toggle Checkmark, F8 Add to ToDo, F9 New Page, F10 New Directory, F11 Delete, F12 Update Next Steps and Search Index).

#### Scenario: Even spacing
- **WHEN** the menu bar of the map window or of an editor window is shown
- **THEN** the gap between each two neighbouring menu names is the same, whatever their lengths
