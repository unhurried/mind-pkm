## ADDED Requirements

### Requirement: Tables in quotes and list items
A table inside a quote or a list item SHALL be edited cell by cell like any other table, its lines keeping the quote's `>` and the item's indentation.

#### Scenario: A table in a list item
- **WHEN** `x` is typed into the first body cell of a table that is a block of a list item
- **THEN** the table's lines keep the item's indentation and the table stays in the item

#### Scenario: A table in a quote
- **WHEN** `y` is typed into a body cell of the table `> | a | b |` / `> | --- | --- |` / `> |  |  |`
- **THEN** the cell holds `y` and every line of the table still starts with `> `
