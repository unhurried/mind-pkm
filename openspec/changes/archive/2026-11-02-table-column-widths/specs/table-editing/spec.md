## ADDED Requirements

### Requirement: Column widths while editing
While the cursor is in a table, the table's columns SHALL have the widths they have when it is rendered, worked out again after every edit from what the cells show. A table that scrolls sideways SHALL keep the scroll position it had when it was clicked, and SHALL scroll to show the cell with the cursor when the cursor moves to a cell out of view.

#### Scenario: Typing widens a column
- **WHEN** text is typed into a cell of a table whose columns are all short, making that cell the widest of the table
- **THEN** that cell's column becomes wider than the others as it is typed

#### Scenario: Same widths when clicked
- **WHEN** a cell of a rendered table is clicked
- **THEN** the table's columns keep their widths and places while the cell is edited

#### Scenario: Tab into a column out of view
- **WHEN** the cursor is in the last visible column of a table that scrolls sideways and Tab is pressed
- **THEN** the table scrolls so that the next column's cell, now holding the cursor, is in view
