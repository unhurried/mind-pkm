## MODIFIED Requirements

### Requirement: Blocks under a list item
Backspace at the start of the text of a list item that follows another item of its list, or that is the first item of a nested list, SHALL make it a block of that item (the previous item, or the item its list is nested in): an empty item SHALL become a new empty paragraph inside that item, where any block can be typed, and an item with text a paragraph of that item. A task item SHALL lose its box together with its bullet, never becoming a plain list item first.

#### Scenario: A table under a list item
- **WHEN** Enter is pressed at the end of the list item `aaa`, then Backspace, then `| bbb | ccc |` is typed and Enter is pressed
- **THEN** the table with the header `bbb`, `ccc` is a block of the item `aaa`, shown below its text at its indentation, with the cursor in its first empty cell

#### Scenario: An item into the item before
- **WHEN** the cursor is at the start of `b` in the list `- a` / `- b` / `- c` and Backspace is pressed
- **THEN** `b` is a second paragraph of the item `a`, and `c` stays the next item of the list

#### Scenario: An empty task item
- **WHEN** Enter is pressed at the end of the task item `- [ ] aaa` and then Backspace is pressed once
- **THEN** the new task item is gone and the cursor is in a new empty paragraph of the item `aaa`

#### Scenario: The first task item of a list
- **WHEN** the cursor is at the start of `done` in the task item `- [x] done` that is the first item of a list at the top level and Backspace is pressed
- **THEN** the block becomes the paragraph `done`
