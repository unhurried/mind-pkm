## MODIFIED Requirements

### Requirement: Outline
Ctrl+O SHALL show and hide an outline beside the document listing the headings, indented by level; clicking one SHALL scroll to that heading. Each editor window SHALL show or hide its own outline; a window that opens SHALL show the outline if it was last shown in any editor window and hide it if it was last hidden, remembered across starts.

#### Scenario: Jumping to a heading
- **WHEN** the outline is shown and `Journal` is clicked in it
- **THEN** the document scrolls so that the heading `Journal` is at the top

#### Scenario: The next window shows the outline
- **WHEN** Ctrl+O shows the outline in the editor of `Trip.md` and `Plan.md` is opened
- **THEN** the editor of `Plan.md` opens with the outline shown

#### Scenario: The next window hides the outline
- **WHEN** the outline is shown in the editors of `Trip.md` and `Plan.md`, Ctrl+O hides it in the editor of `Plan.md` and `Notes.md` is opened
- **THEN** the editor of `Notes.md` opens without the outline and the editor of `Trip.md` keeps it

#### Scenario: Remembered across starts
- **WHEN** the outline was last shown, the application is closed and started again and a page is opened
- **THEN** the page's editor opens with the outline shown
