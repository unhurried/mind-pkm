## MODIFIED Requirements

### Requirement: Opening maps
On start the application SHALL open the map given as the first command-line argument (so `.mm` files can be opened with it from Explorer) unless that argument is a Markdown file (see Starting with a Markdown file), else the last opened map if it still exists, else show a welcome screen with Open and New. File → Open SHALL pick a `.mm` or a `.md` file: a map takes the open map's place, and a Markdown file opens in an editor window (in the built-in editor, whatever Use Built-in Editor says) while the open map stays as it is. File → New SHALL create a map with `config > docDirPath > <path>` and `ToDo` nodes. A map opens with the default filter (see map-filter) and the root centered, unless its last view is restored (see Restoring the view).

#### Scenario: Restart
- **WHEN** the application is closed with a map open and started again without arguments
- **THEN** the same map is opened

#### Scenario: Opening a Markdown file with File → Open
- **WHEN** a map with unsaved changes is open, File → Open is chosen, Don't Save is answered and `Notes.md` is chosen in the file dialog
- **THEN** `Notes.md` opens in an editor window, and the map is still shown with its unsaved changes

### Requirement: Map window and editor windows
The application SHALL have at most one map window and any number of editor windows. Closing the map window (or File → Exit) SHALL end the application, closing every editor window; closing an editor window SHALL close only that window. When the application runs without a map window (see Starting with a Markdown file), closing its last editor window SHALL end the application.

#### Scenario: Closing the map window
- **WHEN** two pages are open in editor windows without unsaved changes and the map window is closed
- **THEN** the application ends and both editor windows close

#### Scenario: Closing the last editor window without a map window
- **WHEN** the application was started with `Notes.md`, `Other.md` was opened from it, and both editor windows are closed
- **THEN** the application ends when the second window closes, and not before

## ADDED Requirements

### Requirement: Starting with a Markdown file
When the first command-line argument is a `.md` file (for example a Markdown file opened with the application from Explorer), the application SHALL open that file in an editor window and SHALL NOT open the map window, in the built-in editor whatever Use Built-in Editor says. The last opened map and the remembered map window SHALL stay as they were. When the file cannot be opened, the application SHALL start as it does without arguments and show the error in the map window.

#### Scenario: Opening a Markdown file from Explorer
- **WHEN** the application is started with `C:\notes\Notes.md` as its argument
- **THEN** `Notes.md` is shown in an editor window and no map window opens

#### Scenario: The last map is kept
- **WHEN** the application is started with `Notes.md`, its editor window is closed, and the application is started again without arguments
- **THEN** the map opened last before is opened, in its window as it was then

#### Scenario: Built-in editor turned off
- **WHEN** Use Built-in Editor is unchecked and the application is started with `Notes.md`
- **THEN** `Notes.md` opens in an editor window of the application, not with the default application

#### Scenario: A file that cannot be opened
- **WHEN** the application is started with a `.md` file that does not exist
- **THEN** the map window opens as at a start without arguments and shows an error naming the file
