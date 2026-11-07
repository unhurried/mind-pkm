## MODIFIED Requirements

### Requirement: Showing and hiding the assets pane
View → Assets (Ctrl+D) SHALL show and hide a pane on the right of the editor window, marked with a check in the menu while shown. Each editor window SHALL show or hide its own pane; a window that opens SHALL show the pane if it was last shown in any editor window and hide it if it was last hidden, remembered across starts. The page's column SHALL narrow to leave room for the pane and, when it is shown too, the outline.

#### Scenario: Showing the pane
- **WHEN** Ctrl+D is pressed in the editor of `Trip.md`
- **THEN** the assets pane appears on the right and View → Assets is checked

#### Scenario: Other windows
- **WHEN** the pane is shown in the editor of `Trip.md` and `Plan.md` is opened
- **THEN** the editor of `Plan.md` opens with the pane shown

#### Scenario: Hidden in the next window
- **WHEN** the pane is shown in the editors of `Trip.md` and `Plan.md`, Ctrl+D hides it in the editor of `Plan.md` and `Notes.md` is opened
- **THEN** the editor of `Notes.md` opens without the pane and the editor of `Trip.md` keeps it

#### Scenario: Remembered across starts
- **WHEN** the pane was last shown, the application is closed and started again and a page is opened
- **THEN** the page's editor opens with the pane shown and listing the page's assets folder
