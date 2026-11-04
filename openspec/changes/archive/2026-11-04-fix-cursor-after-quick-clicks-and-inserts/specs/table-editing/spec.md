## ADDED Requirements

### Requirement: Cursor after Insert Table
After Ctrl+T or Table → Insert Table, the cursor SHALL be in the first header cell of the new table, so that its header is typed first.

#### Scenario: Typing after Insert Table
- **WHEN** Ctrl+T is pressed in a paragraph and `Name` is typed
- **THEN** the new table's first header cell holds `Name` and its other cells are empty

#### Scenario: Typing after Insert Table in a list item
- **WHEN** Ctrl+T is pressed in the list item `a` and `Name` is typed
- **THEN** the table under `a` holds `Name` in its first header cell
