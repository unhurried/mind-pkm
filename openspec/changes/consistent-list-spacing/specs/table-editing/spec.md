## MODIFIED Requirements

### Requirement: Creating a table
Enter after a line that is a table header row (`| a | b |`) SHALL turn it into a table with that header, one empty row and the cursor in its first cell. Ctrl+T and Table → Insert Table SHALL insert a table of two columns, with a header row and two body rows, after the active block; while a list item's text is active, the table SHALL go into that item (see page-editor, Blocks inserted in a list item). A table SHALL always keep its header row, at least one body row and at least one column.

#### Scenario: Header row and Enter
- **WHEN** `| Name | Note |` is typed in an empty paragraph and Enter is pressed
- **THEN** the source becomes a table with the header `Name`, `Note`, a delimiter row and one empty row, and the cursor is in the first empty cell

#### Scenario: Insert Table in a list item
- **WHEN** the cursor is in the item `b` of the list `- a` / `- b` / `- c` and Ctrl+T is pressed
- **THEN** the table is a block of the item `b`, indented under it, and `- c` is still the next item of the same list
