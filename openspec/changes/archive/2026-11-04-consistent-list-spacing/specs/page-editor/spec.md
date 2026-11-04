## ADDED Requirements

### Requirement: Blank lines in edited lists
A list changed by an edit SHALL be written in one form for all of its items: tight, with no blank line between its items nor before the blocks inside its items, or loose, with exactly one blank line in each of these places. A nested list SHALL take its own form. The form SHALL be applied after an editing command and when the cursor leaves a block whose text was edited; lists no edit changed, and pages only opened or saved, SHALL keep their source.

#### Scenario: A new item in a list with mixed blank lines
- **WHEN** the source is `* a`, a blank line, `* b`, `* c`, the list holds only single-line items, and Enter is pressed at the end of `c` and `d` is typed
- **THEN** the source reads `* a` / `* b` / `* c` / `* d` with no blank lines

#### Scenario: A nested list keeps its own form
- **WHEN** an item of the outer list gets a second paragraph while its nested list `* x` / `* y` has no blank lines
- **THEN** the outer list's items and blocks are separated by one blank line each, and `* x` / `* y` stay without a blank line between them

#### Scenario: A list nobody edited
- **WHEN** a page holds a list `* a`, a blank line, `* b`, `* c` and text is typed into a paragraph after the list
- **THEN** the list's source is unchanged

#### Scenario: A list inside a quote
- **WHEN** a list inside a quote is written loose
- **THEN** each blank line between its items keeps the quote's `>`

### Requirement: Loose only where blank lines are needed
An edited list SHALL be written loose only when a block inside one of its items would not stay that block without a blank line before it - a paragraph, an image or math on lines of its own, a `---` rule - and tight otherwise: code blocks, tables, quotes, headings and nested lists inside items SHALL keep a list tight. A list SHALL become tight again when its last such block goes.

#### Scenario: A code block under an item keeps the list tight
- **WHEN** in the list `* a` / `* b` / `* c` Enter is pressed at the end of `b`, then Backspace, then ```` ``` ```` is typed and Enter is pressed, and `x` is typed in the code block
- **THEN** the source reads `* a`, `* b`, the code block holding `x` indented under `b`, `* c`, with no blank lines, and the items are shown close together

#### Scenario: A paragraph under an item makes the list loose
- **WHEN** in the list `* a` / `* b` / `* c` Enter is pressed at the end of `b`, then Backspace, `more` is typed and the cursor leaves the paragraph
- **THEN** the source reads `* a`, `* b`, `more` indented under `b`, `* c`, with one blank line between every two of them, and every item is shown with room around it

#### Scenario: A rule under an item
- **WHEN** a horizontal rule is inserted while the item `b` of the tight list `* a` / `* b` is active
- **THEN** the rule is written `---` after a blank line and the list is written loose, so that `b` stays an item's text and does not become a heading

#### Scenario: Back to tight
- **WHEN** a list is loose only because of the paragraph `more` under its item `b`, and `more` is deleted and the cursor leaves the paragraph
- **THEN** the list is written tight again

#### Scenario: A loose list that needs no blank lines
- **WHEN** a list of single-line items written with blank lines between them is edited with Enter
- **THEN** the list is written tight

### Requirement: Blocks inserted in a list item
Inserting a code block, a math block, a horizontal rule or a table (also pasting cells as a table) while a list item's text is the active block SHALL put the new block into that item, right below its text and before the item's other blocks, at the item's indentation; the list SHALL stay one list.

#### Scenario: A code block inserted in an item
- **WHEN** the cursor is in the item `b` of the list `* a` / `* b` / `* c` and Ctrl+Shift+K is pressed
- **THEN** an empty code block is a block of the item `b`, the cursor is in it, `* c` is still an item of the same list, and the list stays tight

#### Scenario: An ordered list stays numbered
- **WHEN** the cursor is in the item `2.` of the list `1.` / `2.` / `3.` and Ctrl+T is pressed
- **THEN** the table is a block of the item `2.`, and the items keep the numbers 1, 2 and 3 of one list

## MODIFIED Requirements

### Requirement: Blocks under a list item
Backspace at the start of the text of a list item that follows another item of its list, or that is the first item of a nested list, SHALL make it a block of that item (the previous item, or the item its list is nested in): an empty item SHALL become a new empty paragraph inside that item, where any block can be typed, and an item with text a paragraph of that item. A task item SHALL lose its box first.

#### Scenario: A table under a list item
- **WHEN** Enter is pressed at the end of the list item `aaa`, then Backspace, then `| bbb | ccc |` is typed and Enter is pressed
- **THEN** the table with the header `bbb`, `ccc` is a block of the item `aaa`, shown below its text at its indentation, with the cursor in its first empty cell, and no blank line is written before the table

#### Scenario: An item into the item before
- **WHEN** the cursor is at the start of `b` in the list `- a` / `- b` / `- c` and Backspace is pressed
- **THEN** `b` is a second paragraph of the item `a`, `c` stays the next item of the list, and the list is written loose: one blank line before `b` and one before `- c`
