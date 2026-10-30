## ADDED Requirements

### Requirement: Enter after the text of a list item with blocks below
Enter at the end of a list item's text SHALL move the nested items right below that text under the new item. Other blocks below the text (paragraphs, code blocks, quotes) SHALL stay the item's, and the new item SHALL follow them.

#### Scenario: An item with a code block below its text
- **WHEN** Enter is pressed at the end of the item `* aaa` that has a code block below its text, and `b` is typed
- **THEN** the code block stays in the item `aaa`, and a new item `b` follows it

#### Scenario: An item with a nested list below its text
- **WHEN** Enter is pressed at the end of the item `* a` that has the nested item `* b` below its text, and `x` is typed
- **THEN** a new item `x` follows `a`, and `b` is nested in `x`

### Requirement: Down below code ending a quote or list item
Down at the last line of a code block that is the last block of a list item or quote SHALL start a new empty paragraph after it inside that item or quote, where any block can be typed. Down in that paragraph while it is empty SHALL move on to the next block, leaving the page unchanged.

#### Scenario: A block after code ending a list item
- **WHEN** Down is pressed at the last line of a code block that is the last block of a list item followed by a paragraph outside the list, and `z` is typed
- **THEN** `z` is a new paragraph of the list item, after the code block, indented like it

#### Scenario: Moving on past the item
- **WHEN** Down is pressed twice at the last line of a code block that is the last block of a list item followed by the paragraph `next`
- **THEN** the cursor is in `next` and the page is unchanged
