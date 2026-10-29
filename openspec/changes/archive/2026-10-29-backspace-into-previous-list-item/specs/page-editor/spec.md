## MODIFIED Requirements

### Requirement: Backspace and Delete across blocks
Backspace at the start of the text of a heading, quote or the first list item at the top level SHALL remove its block marker (`# `, `- `, `1. `, `> `), making the block a paragraph. Backspace at the very start of a paragraph SHALL join it to the end of the previous paragraph, heading or list item, and otherwise move the cursor to the end of the previous block. Delete at the very end of a block SHALL join the next block's text in the same way.

#### Scenario: Removing a heading marker
- **WHEN** the cursor is at the start of the heading `Plan` (source `## Plan`) and Backspace is pressed
- **THEN** the block becomes the paragraph `Plan`

#### Scenario: Joining paragraphs
- **WHEN** the cursor is at the start of the paragraph `two` that follows the paragraph `one` and Backspace is pressed
- **THEN** the two paragraphs become one paragraph `onetwo` with the cursor between them

## ADDED Requirements

### Requirement: Blocks under a list item
Backspace at the start of the text of a list item that follows another item of its list, or that is the first item of a nested list, SHALL make it a block of that item (the previous item, or the item its list is nested in): an empty item SHALL become a new empty paragraph inside that item, where any block can be typed, and an item with text a paragraph of that item. A task item SHALL lose its box first.

#### Scenario: A table under a list item
- **WHEN** Enter is pressed at the end of the list item `aaa`, then Backspace, then `| bbb | ccc |` is typed and Enter is pressed
- **THEN** the table with the header `bbb`, `ccc` is a block of the item `aaa`, shown below its text at its indentation, with the cursor in its first empty cell

#### Scenario: An item into the item before
- **WHEN** the cursor is at the start of `b` in the list `- a` / `- b` / `- c` and Backspace is pressed
- **THEN** `b` is a second paragraph of the item `a`, and `c` stays the next item of the list
