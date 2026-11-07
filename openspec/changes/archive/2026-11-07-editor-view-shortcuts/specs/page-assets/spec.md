## MODIFIED Requirements

### Requirement: Showing and hiding the assets pane
View → Assets (Ctrl+D) SHALL show and hide a pane on the right of the editor window, marked with a check in the menu while shown. Each editor window SHALL show or hide its own pane, and the pane SHALL be hidden when a window opens. The page's column SHALL narrow to leave room for the pane and, when it is shown too, the outline.

#### Scenario: Showing the pane
- **WHEN** Ctrl+D is pressed in the editor of `Trip.md`
- **THEN** the assets pane appears on the right and View → Assets is checked

#### Scenario: Other windows
- **WHEN** the pane is shown in the editor of `Trip.md` and `Plan.md` is opened
- **THEN** the editor of `Plan.md` opens without the pane and the editor of `Trip.md` keeps it
