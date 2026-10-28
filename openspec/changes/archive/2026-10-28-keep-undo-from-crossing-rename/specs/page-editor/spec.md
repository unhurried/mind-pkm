## MODIFIED Requirements

### Requirement: PKM commands on open pages
Edit (F2) renaming a page or a directory, Add Assets and Delete (F11) SHALL first save the open editors of the affected files that have unsaved changes. Afterwards a renamed page's editor SHALL show the renamed file with its updated text, Add Assets' page SHALL be reloaded, and the editors of deleted files SHALL close. When a rename changed the text of an open page, Undo in its editor SHALL NOT go back across that change.

#### Scenario: Renaming an open page
- **WHEN** page `A` is open with unsaved changes and renamed to `B` with F2
- **THEN** the changes are saved, `B.md` holds them with `A.assets` references renamed, and the editor shows `B.md`

#### Scenario: Undo after renaming an open page
- **WHEN** page `A` with a reference `A.assets` is open, renamed to `B` with F2 and Undo is pressed in its editor
- **THEN** the page still references `B.assets`

#### Scenario: Deleting an open page
- **WHEN** page `P` is open in an editor and deleted with F11
- **THEN** its editor window closes without asking
