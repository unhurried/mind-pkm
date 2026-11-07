## MODIFIED Requirements

### Requirement: Opening files from an editor
File → Open... (Ctrl+Shift+O) in an editor window SHALL pick a `.md` file in a file dialog that starts in the folder of the window's file, and open it in an editor window of its own (bringing its window to the front when it is open already), in the built-in editor whatever Use Built-in Editor says. It SHALL work alike with and without a map window.

#### Scenario: Opening another file
- **WHEN** Ctrl+Shift+O is pressed in the editor of `Notes.md` and `Other.md` is chosen
- **THEN** `Other.md` opens in a new editor window and the window of `Notes.md` stays open

#### Scenario: A file that is open already
- **WHEN** `Other.md` is open in an editor window, Ctrl+Shift+O is pressed in the editor of `Notes.md` and `Other.md` is chosen
- **THEN** the window of `Other.md` comes to the front and no second window opens

#### Scenario: Cancelling the dialog
- **WHEN** Ctrl+Shift+O is pressed in an editor window and the file dialog is cancelled
- **THEN** nothing opens and the page is unchanged

### Requirement: Errors of opening from an editor
When a file cannot be opened from an editor window - chosen with File → Open... or a link that is followed - the error SHALL be shown in that editor window, not in the map window.

#### Scenario: A file that is not UTF-8
- **WHEN** Ctrl+Shift+O is pressed in the editor of `Notes.md` and a `.md` file encoded in Shift_JIS is chosen
- **THEN** the editor of `Notes.md` shows an error naming the file and no editor window opens

#### Scenario: A link to a missing page
- **WHEN** `[see](Gone.md)` is Ctrl+clicked and `Gone.md` does not exist
- **THEN** the editor window holding the link shows an error naming `Gone.md`

### Requirement: Selection helpers
Edit → Select Word SHALL select the word at the cursor (it has no shortcut; a double click selects a word too); Ctrl+L SHALL select the line (in a table, the row); Ctrl+E SHALL select the formatted span the cursor is in (in a table, the cell).

#### Scenario: Selecting a styled span
- **WHEN** the cursor is inside `**bold text**` and Ctrl+E is pressed
- **THEN** `bold text` is selected

### Requirement: Outline
Ctrl+O SHALL show and hide an outline beside the document listing the headings, indented by level; clicking one SHALL scroll to that heading.

#### Scenario: Jumping to a heading
- **WHEN** the outline is shown and `Journal` is clicked in it
- **THEN** the document scrolls so that the heading `Journal` is at the top
