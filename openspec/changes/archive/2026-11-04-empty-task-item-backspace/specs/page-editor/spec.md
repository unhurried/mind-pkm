## MODIFIED Requirements

### Requirement: Blocks under a list item
Backspace at the start of the text of a list item that follows another item of its list, or that is the first item of a nested list, SHALL make it a block of that item (the previous item, or the item its list is nested in): an empty item SHALL become a new empty paragraph inside that item, where any block can be typed, and an item with text a paragraph of that item.

#### Scenario: A table under a list item
- **WHEN** Enter is pressed at the end of the list item `aaa`, then Backspace, then `| bbb | ccc |` is typed and Enter is pressed
- **THEN** the table with the header `bbb`, `ccc` is a block of the item `aaa`, shown below its text at its indentation, with the cursor in its first empty cell, and no blank line is written before the table

#### Scenario: An item into the item before
- **WHEN** the cursor is at the start of `b` in the list `- a` / `- b` / `- c` and Backspace is pressed
- **THEN** `b` is a second paragraph of the item `a`, `c` stays the next item of the list, and the list is written loose: one blank line before `b` and one before `- c`

## ADDED Requirements

### Requirement: Backspace in a task item
Backspace at the start of the text of a task item with text SHALL remove only its box, leaving a plain list item. Backspace at the start of an empty task item SHALL remove its box and its bullet with one press, doing what Backspace does at the start of an empty plain item (see Blocks under a list item, and Backspace and Delete across blocks): no empty item without its box SHALL be left in between.

#### Scenario: A task item with text
- **WHEN** the cursor is at the start of `call` in the task item `- [ ] call` and Backspace is pressed
- **THEN** the item becomes the plain list item `- call`

#### Scenario: An empty task item
- **WHEN** Enter is pressed at the end of the task item `- [ ] a`, which starts the empty task item `- [ ] `, and Backspace is pressed
- **THEN** the empty task item is gone and the cursor is in a new empty paragraph inside the item `a`, where `more` typed becomes its second paragraph

#### Scenario: An empty task item at the top of a page
- **WHEN** the page holds only the empty task item `- [ ] ` and Backspace is pressed at its start
- **THEN** the page holds an empty paragraph
