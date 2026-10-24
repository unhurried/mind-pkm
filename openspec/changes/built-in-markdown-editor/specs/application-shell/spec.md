## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Per-user settings
The application SHALL keep its settings in `%APPDATA%\mind-pkm\`: the last opened map, the window's size, position and maximized state, how each map was last shown (zoom level, scroll position, selected nodes; the map file itself is not changed for it), an optional `template.md` that replaces the built-in New Page template, whether pages open in the built-in editor, the editor windows' size, zoom level and theme, whether the editor pairs brackets, the image insertion options, and an optional `export.css` for page export. A settings file that is not there SHALL be read from `%APPDATA%\freeplane-pkm\`, where versions under the former name kept it.

#### Scenario: Custom page template
- **WHEN** `%APPDATA%\mind-pkm\template.md` exists
- **THEN** New Page creates pages from it instead of the built-in template

#### Scenario: Settings from the former name
- **WHEN** only `%APPDATA%\freeplane-pkm\` holds the last opened map and a `template.md`
- **THEN** that map is opened on start and New Page uses that template

#### Scenario: Editor theme kept
- **WHEN** the dark theme is chosen in an editor window and the application is started again
- **THEN** pages open in the dark theme
