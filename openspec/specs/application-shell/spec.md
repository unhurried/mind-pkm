# application-shell Specification

## Purpose
The application as a standalone Windows program: starting it, opening and saving maps, the menus and shortcuts, the dialogs, and the per-user settings.

## Requirements

### Requirement: Standalone Windows executable
The application SHALL be a single Windows executable that needs no installer, runtime, Freeplane or Java, with everything (including the search dictionary) embedded. It SHALL show no console window and SHALL carry the application icon in the executable and on its window.

#### Scenario: First run
- **WHEN** the executable is copied to a machine and started
- **THEN** it runs without installing anything else

### Requirement: Opening maps
On start the application SHALL open the map given as the first command-line argument (so `.mm` files can be opened with it from Explorer), else the last opened map if it still exists, else show a welcome screen with Open and New. File → Open SHALL pick a `.mm` file; File → New SHALL create a map with `config > docDirPath > <path>` and `ToDo` nodes. The map opens with the archive-hiding filter and the root centered, unless its last view is restored (see Restoring the view).

#### Scenario: Restart
- **WHEN** the application is closed with a map open and started again without arguments
- **THEN** the same map is opened

### Requirement: Restoring the view
A map opened again SHALL be shown with the zoom level, scroll position and selected nodes it was last shown with, remembered when the application exits, when another map takes its place, and when the map is saved. Selected nodes that no longer exist SHALL be left out (the root is selected if none is left); a selected node hidden by the filter or a fold SHALL hand its selection to its nearest shown ancestor.

#### Scenario: Restart keeps the view
- **WHEN** the map is zoomed to 150%, scrolled, a node is selected, and the application is closed and started again
- **THEN** the map is shown at 150% at the same scroll position with the same node selected

#### Scenario: Selected node gone
- **WHEN** the node selected when the map was last shown no longer exists in the map file
- **THEN** the map opens with the root selected

### Requirement: Saving and unsaved changes
The map SHALL be written only when saved: Ctrl+S (Save As for a new map) or Ctrl+Shift+S. The window title SHALL show the map name with `*` while there are unsaved changes. Creating or opening another map, or closing the window, with unsaved changes SHALL ask whether to save, discard or cancel, and SHALL go on once the map is saved, also after Save As for a new map.

#### Scenario: Closing with unsaved changes
- **WHEN** the window is closed with unsaved changes and Cancel is chosen
- **THEN** the application stays open with the changes

#### Scenario: Saving a new map before creating another
- **WHEN** a map that has never been saved has changes, New is chosen, Save is answered and a file is chosen in Save As
- **THEN** the map is written to that file and the new map is shown

#### Scenario: Save As cancelled
- **WHEN** a map that has never been saved has changes, the window is closed, Save is answered and Save As is cancelled
- **THEN** the application stays open with the changes

### Requirement: Menus and shortcuts
A menu bar SHALL offer File, Edit, View and PKM menus listing every command with its shortcut. The PKM commands SHALL keep the add-on's function-key assignments (F1 Open, F2 Edit, F3 Fold One Level, F4 Unfold One Level, F5 Filter, F6 Search, F7 Toggle Checkmark, F8 Add to ToDo, F9 New Page, F10 New Directory, F11 Delete, F12 Update Next Steps and Search Index). While the search keyword box has the keyboard, only the function keys and Ctrl+F SHALL reach the map.

#### Scenario: Typing in the search box
- **WHEN** Tab or Delete is pressed while typing a search keyword
- **THEN** no node is created or deleted

#### Scenario: Ctrl+N creates a page
- **WHEN** Ctrl+N is pressed with a node selected
- **THEN** the New Page command runs as with F9

### Requirement: Ctrl shortcuts for PKM commands
Some PKM commands SHALL also be reachable by Ctrl shortcuts: Ctrl+O (Open), Ctrl+Shift+F (Filter), Ctrl+F (Search), Ctrl+N (New Page) and Ctrl+Shift+N (New Directory). File → New and File → Open SHALL have no shortcut.

#### Scenario: Ctrl+O opens the page
- **WHEN** Ctrl+O is pressed with a page node selected
- **THEN** the page is opened as with F1

#### Scenario: Map commands have no Ctrl+N
- **WHEN** the File menu is opened
- **THEN** New and Open are listed without a shortcut

### Requirement: View menu and zoom level
The View menu SHALL offer Zoom In (Ctrl+Plus), Zoom Out (Ctrl+Minus) and Reset Zoom (Ctrl+0), and the status bar SHALL show the zoom level.

#### Scenario: Zoom level
- **WHEN** the map is zoomed to 150%
- **THEN** the status bar shows 150%

### Requirement: Dialogs
Errors, confirmations and text input SHALL be shown as modal dialogs inside the window; several messages SHALL be shown one after another. Enter SHALL confirm and Esc SHALL cancel, also while a dialog's text box has the keyboard.

#### Scenario: Errors for several nodes
- **WHEN** New Page fails for two selected nodes
- **THEN** two error messages are shown one after the other

### Requirement: Per-user settings
The application SHALL keep its settings in `%APPDATA%\mind-pkm\`: the last opened map, the window's size, position and maximized state, how each map was last shown (zoom, scroll position, selection; not in the map file), the page editor's settings, and optional `template.md` (the New Page template) and `export.css` (the export style sheet) files. A settings file that is not there SHALL be read from `%APPDATA%\freeplane-pkm\`, where versions under the former name kept it.

#### Scenario: Custom page template
- **WHEN** `%APPDATA%\mind-pkm\template.md` exists
- **THEN** New Page creates pages from it instead of the built-in template

#### Scenario: Settings from the former name
- **WHEN** only `%APPDATA%\freeplane-pkm\` holds the last opened map and a `template.md`
- **THEN** that map is opened on start and New Page uses that template

#### Scenario: Editor theme kept
- **WHEN** the dark theme is chosen in an editor window and the application is started again
- **THEN** pages open in the dark theme

### Requirement: Arrow pointer
The mouse pointer SHALL be the normal arrow everywhere in every window of the application - over the map and its nodes, the editor's text, buttons, menus, text boxes and open drop-down lists - and SHALL NOT change shape while dragging the map.

#### Scenario: Pointer over a node
- **WHEN** the pointer is moved over a node or a button
- **THEN** it stays the normal arrow

#### Scenario: Pointer over a text box
- **WHEN** the pointer is moved over the search keyword box
- **THEN** it stays the normal arrow

#### Scenario: Pointer over a page's text
- **WHEN** the pointer is moved over the text of a page in an editor window
- **THEN** it stays the normal arrow

### Requirement: Application name
The application SHALL be named Mind PKM: the window title SHALL end with "Mind PKM", the welcome screen SHALL show it, and the executable SHALL be `mind-pkm.exe`.

#### Scenario: Window title
- **WHEN** the map `notes.mm` is open without unsaved changes
- **THEN** the window title is `notes.mm - Mind PKM`

### Requirement: Restoring the window
The application SHALL open its window with the size, position and maximized state it had when it last exited; without a remembered window it SHALL open at 1280×800. A window that was maximized SHALL open maximized and return to the size and position it had before being maximized when it is restored. The size and position of a minimized window SHALL NOT be remembered.

#### Scenario: Restart keeps the window
- **WHEN** the window is resized to 900×600, moved, and the application is closed and started again
- **THEN** the window opens at 900×600 at the same position

#### Scenario: Restart while maximized
- **WHEN** the application is closed while its window is maximized and started again
- **THEN** the window opens maximized, and restoring it returns to the size and position it had before being maximized

### Requirement: Map window and editor windows
The application SHALL have one map window and any number of editor windows. Closing the map window (or File → Exit) SHALL end the application, closing every editor window; closing an editor window SHALL close only that window.

#### Scenario: Closing the map window
- **WHEN** two pages are open in editor windows without unsaved changes and the map window is closed
- **THEN** the application ends and both editor windows close

### Requirement: Exiting with unsaved pages
Ending the application while editor windows have unsaved changes SHALL ask, after any question about the map, whether to save them, naming the pages: Save saves every changed page and ends the application (unless a page cannot be saved, which is reported), Don't Save ends it discarding the changes, Cancel keeps the application open.

#### Scenario: Cancelling the exit
- **WHEN** a page has unsaved changes, the map window is closed and Cancel is chosen
- **THEN** the application stays open with the page's changes

#### Scenario: Saving on exit
- **WHEN** two pages have unsaved changes, the map window is closed and Save is chosen
- **THEN** both pages are written and the application ends
