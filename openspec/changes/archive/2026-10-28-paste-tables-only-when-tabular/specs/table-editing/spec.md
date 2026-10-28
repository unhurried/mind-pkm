## MODIFIED Requirements

### Requirement: Copying and pasting cells
Copying a cell range, row or column (also from the right-click menu) SHALL put it on the clipboard as tab-separated text and as an HTML table, and Copy as Markdown as a Markdown table. Pasting tab-separated text or an HTML table into a table SHALL overwrite the cells from the cursor's cell to the right and down, adding rows and columns as needed. Pasting either outside a table SHALL insert a new table. Text SHALL count as tab-separated only when every row has the same number of cells and not every row starts with a tab; other text with tabs SHALL be pasted as text. HTML that holds other content besides a table SHALL be pasted as Markdown in full, the table as a Markdown table in its place.

#### Scenario: Pasting from a spreadsheet
- **WHEN** a 3×2 range copied from a spreadsheet is pasted into the last row's first cell of a 2-column table
- **THEN** the table grows to hold the 3 pasted rows

#### Scenario: Pasting tab-indented code
- **WHEN** the plain text `def g():` / `<tab>return 1` is pasted into a paragraph
- **THEN** it is inserted as text, not as a table

#### Scenario: Pasting part of a web page with a table
- **WHEN** HTML with a paragraph, a list and a table is pasted
- **THEN** the paragraph, the list and the table are all inserted as Markdown, in that order
