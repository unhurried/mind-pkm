## MODIFIED Requirements

### Requirement: Enter and Shift+Enter
Enter SHALL end the paragraph and start a new one after it. At the start of a nonempty rendered heading with a collapsed selection, Enter SHALL insert an empty paragraph above the heading and keep the caret at the start of the heading. At the end of a heading, Enter SHALL start a new paragraph after it. Shift+Enter SHALL break the line inside a paragraph.

#### Scenario: New paragraph
- **WHEN** the cursor is in the middle of `onetwo` (after `one`) and Enter is pressed
- **THEN** the source reads `one`, a blank line, `two`, with the cursor at the start of `two`

#### Scenario: Line break in a paragraph
- **WHEN** Shift+Enter is pressed after `one` in `onetwo`
- **THEN** the paragraph's source reads `one` and `two` on two lines and it is shown with a line break

#### Scenario: Enter at the start of a heading
- **WHEN** the caret is before the first character of `## Plan` and Enter is pressed
- **THEN** an empty paragraph appears above the heading, which keeps its level and text
- **AND** the caret stays before `Plan`, so typing `New ` makes the heading read `## New Plan`

#### Scenario: Typing in the empty paragraph above a heading
- **WHEN** Enter is pressed at the start of a heading and the user presses Up or clicks the empty paragraph above it
- **THEN** the caret enters the empty paragraph and typing adds text there

#### Scenario: Saving an empty paragraph above a heading
- **WHEN** an empty paragraph is created with Enter at the start of a heading and the page is saved and reopened
- **THEN** the empty paragraph is visible above the heading

#### Scenario: Undoing an empty paragraph above a heading
- **WHEN** Enter is pressed at the start of a heading and is immediately undone
- **THEN** the empty paragraph disappears and the caret returns to its original position

#### Scenario: Enter at the end of a heading
- **WHEN** Enter is pressed after `Plan` in the rendered heading `## Plan`
- **THEN** the caret is in a new empty paragraph below the heading

### Requirement: Blank lines in edited lists
An edited list SHALL use one form for all its items: tight, with no blank lines between items or before blocks inside items, or loose, with one blank line in each place except where blank lines represent an empty paragraph before a heading. A nested list SHALL take its own form. The form SHALL be applied after an editing command and when the caret leaves a block whose text was edited. Lists no edit changed, and pages only opened or saved, SHALL keep their source.

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

#### Scenario: An empty paragraph before a heading inside an item
- **WHEN** Enter is pressed at the start of a rendered heading under a list item's text
- **THEN** the empty paragraph and the heading are inside the same item, and the caret stays at the start of the heading

### Requirement: Loose only where blank lines are needed
An edited list SHALL be loose when an item contains an empty paragraph before a heading or when a block needs a preceding blank line to stay distinct, such as a paragraph, an image or math on its own lines, or a `---` rule. It SHALL otherwise be tight, including when code blocks, tables, quotes, headings or nested lists can remain distinct without blank lines.

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
