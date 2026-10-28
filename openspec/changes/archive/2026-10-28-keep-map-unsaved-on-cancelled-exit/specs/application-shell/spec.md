## MODIFIED Requirements

### Requirement: Exiting with unsaved pages
Ending the application while editor windows have unsaved changes SHALL ask, after any question about the map, whether to save them, naming the pages: Save saves every changed page and ends the application (unless a page cannot be saved, which is reported), Don't Save ends it discarding the changes, Cancel keeps the application open.

#### Scenario: Cancelling the exit
- **WHEN** a page has unsaved changes, the map window is closed and Cancel is chosen
- **THEN** the application stays open with the page's changes

#### Scenario: Saving on exit
- **WHEN** two pages have unsaved changes, the map window is closed and Save is chosen
- **THEN** both pages are written and the application ends

#### Scenario: Cancelling the exit after discarding the map's changes
- **WHEN** the map and a page have unsaved changes, the map window is closed, Don't Save is answered for the map and Cancel for the pages
- **THEN** the application stays open, the map is still shown as having unsaved changes and closing the window again asks about the map
