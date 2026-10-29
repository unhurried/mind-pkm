## MODIFIED Requirements

### Requirement: Editing cells
While the cursor is in a table, the table SHALL be shown as a grid of rendered cells; the cursor's cell SHALL be edited in its rendered form, showing as Markdown only the inline elements touching the cursor. A cell SHALL accept links, bold, italic and strikethrough. Shift+Enter in a cell SHALL break the line, stored as `<br>`; a `|` typed in a cell SHALL be stored as `\|`. Block elements SHALL NOT be created inside a cell. A table inside a quote or a list item SHALL be edited the same way, its lines keeping the quote's `>` and the item's indentation.

#### Scenario: Line break in a cell
- **WHEN** Shift+Enter is pressed between `a` and `b` in a cell
- **THEN** the cell shows two lines and the file stores the cell as `a<br>b`

#### Scenario: Pipe in a cell
- **WHEN** `x|y` is typed into a cell
- **THEN** the file stores the cell as `x\|y`

#### Scenario: Bold text in the cell being edited
- **WHEN** the cursor is in a cell holding `**done** today`, after `today`
- **THEN** `done` is shown in bold without asterisks

#### Scenario: A table in a list item
- **WHEN** `x` is typed into the first body cell of a table that is a block of a list item
- **THEN** the table's lines keep the item's indentation and the table stays in the item

#### Scenario: A table in a quote
- **WHEN** `y` is typed into a body cell of the table `> | a | b |` / `> | --- | --- |` / `> |  |  |`
- **THEN** the cell holds `y` and every line of the table still starts with `> `
