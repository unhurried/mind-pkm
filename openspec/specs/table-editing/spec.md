# table-editing Specification

## Purpose
Editing GitHub-flavored Markdown tables in the built-in editor cell by cell, with commands for rows, columns, alignment, size and ranges of cells, while the file keeps a plain Markdown table.

## Requirements

### Requirement: Creating a table
Enter after a line that is a table header row (`| a | b |`) SHALL turn it into a table with that header, one empty row and the cursor in its first cell. Ctrl+T and Table → Insert Table SHALL insert a table with a header row and two columns and two rows after the active block. A table SHALL always keep its header row, at least one body row and at least one column.

#### Scenario: Header row and Enter
- **WHEN** `| Name | Note |` is typed in an empty paragraph and Enter is pressed
- **THEN** the source becomes a table with the header `Name`, `Note`, a delimiter row and one empty row, and the cursor is in the first empty cell

### Requirement: Editing cells
While the cursor is in a table, the table SHALL be shown as a grid of rendered cells; the cursor's cell SHALL be edited in its rendered form, showing as Markdown only the inline elements touching the cursor. A cell SHALL accept links, bold, italic and strikethrough. Shift+Enter in a cell SHALL break the line, stored as `<br>`; a `|` typed in a cell SHALL be stored as `\|`. Block elements SHALL NOT be created inside a cell.

#### Scenario: Line break in a cell
- **WHEN** Shift+Enter is pressed between `a` and `b` in a cell
- **THEN** the cell shows two lines and the file stores the cell as `a<br>b`

#### Scenario: Pipe in a cell
- **WHEN** `x|y` is typed into a cell
- **THEN** the file stores the cell as `x\|y`

#### Scenario: Bold text in the cell being edited
- **WHEN** the cursor is in a cell holding `**done** today`, after `today`
- **THEN** `done` is shown in bold without asterisks

### Requirement: Moving between cells
Tab SHALL move to the next cell, from a row's last cell to the next row's first; Shift+Tab SHALL move back the same way. Tab in the last cell of the last row SHALL add a row and move to its first cell. Enter SHALL move to the cell below, adding a row from the last row. The cell moved to SHALL have its whole text selected.

#### Scenario: Tab at the end of the table
- **WHEN** the cursor is in the last cell of the last row and Tab is pressed
- **THEN** a new empty row is added and the cursor is in its first cell

### Requirement: Rows and columns
Ctrl+Enter SHALL insert an empty row below the cursor's row and Ctrl+Shift+Backspace SHALL delete the cursor's row. Rows SHALL also be inserted above or below and deleted, and columns inserted left or right and deleted, from the Table menu and the cell's right-click menu. Alt+Up/Down SHALL move the row and Alt+Left/Right the column, also offered in both menus.

#### Scenario: Deleting a row
- **WHEN** the cursor is in the second body row of a three-row table and Ctrl+Shift+Backspace is pressed
- **THEN** the table has two body rows and the cursor is in the row that followed

#### Scenario: Moving a column
- **WHEN** the cursor is in the first column and Alt+Right is pressed
- **THEN** the first and second columns change places, alignment included

### Requirement: Alignment
The alignment of the cursor's column, or of every column of a selected cell range, SHALL be set to left, center or right from the Table menu, the right-click menu and the table's toolbar, and written to the delimiter row (`:---`, `:---:`, `---:`).

#### Scenario: Centering two columns
- **WHEN** a cell range spanning columns 1 and 2 is selected and Align Center is chosen
- **THEN** the delimiter row reads `:---:` for both columns

### Requirement: Table toolbar
While the cursor is in a table, a toolbar SHALL be shown above it with the alignment buttons, a resize control taking the number of body rows and columns, and a button deleting the whole table. Resizing SHALL keep the existing cells that still fit and add empty ones.

#### Scenario: Resizing
- **WHEN** a 2×2 table is resized to 3 rows and 3 columns
- **THEN** the old cells keep their text and the new ones are empty

### Requirement: Selecting cells
Ctrl+L SHALL select the cursor's row and Ctrl+E its cell. Dragging over cells, or Shift+arrow keys at the edge of a cell's text, SHALL select a rectangular range of cells, shown highlighted. Delete or Backspace on a range SHALL empty its cells.

#### Scenario: Clearing a range
- **WHEN** a 2×2 range is selected and Delete is pressed
- **THEN** the four cells are empty and the table keeps its size

### Requirement: Copying and pasting cells
Copying a cell range, row or column (also from the right-click menu) SHALL put it on the clipboard as tab-separated text and as an HTML table, and Copy as Markdown as a Markdown table. Pasting tab-separated text or an HTML table into a table SHALL overwrite the cells from the cursor's cell to the right and down, adding rows and columns as needed. Pasting either outside a table SHALL insert a new table.

#### Scenario: Pasting from a spreadsheet
- **WHEN** a 3×2 range copied from a spreadsheet is pasted into the last row's first cell of a 2-column table
- **THEN** the table grows to hold the 3 pasted rows

### Requirement: What pasted text becomes a table
Text SHALL count as tab-separated only when every row has the same number of cells and not every row starts with a tab; other text with tabs SHALL be pasted as text. HTML that holds other content besides a table SHALL be pasted as Markdown in full, the table as a Markdown table in its place.

#### Scenario: Pasting tab-indented code
- **WHEN** the plain text `def g():` / `<tab>return 1` is pasted into a paragraph
- **THEN** it is inserted as text, not as a table

#### Scenario: Pasting part of a web page with a table
- **WHEN** HTML with a paragraph, a list and a table is pasted
- **THEN** the paragraph, the list and the table are all inserted as Markdown, in that order

### Requirement: Table source
A table changed by the editor SHALL be written with a leading and trailing pipe on every row and its columns padded to equal width (counting East Asian wide characters double); a table that is not changed SHALL stay as written.

#### Scenario: Written table
- **WHEN** a cell of `|a|b|` / `|-|-|` / `|1|2|` is changed to `xyz`
- **THEN** the rows are written `| xyz | b   |`, `| --- | --- |`, `| 1   | 2   |` style with aligned pipes
