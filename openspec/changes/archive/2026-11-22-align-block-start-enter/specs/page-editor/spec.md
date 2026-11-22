## MODIFIED Requirements

### Requirement: Enter and Shift+Enter
At the start of a nonempty rendered paragraph or heading with a collapsed selection, Enter SHALL insert an empty paragraph above it and keep the caret with the text. Elsewhere in a paragraph, Enter SHALL split it at the caret and activate the second paragraph. Enter at the end of a heading SHALL start a new paragraph below it. Shift+Enter SHALL break the line inside a paragraph.

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

#### Scenario: Enter at the start of a paragraph or quote
- **WHEN** Enter is pressed before the first character of a nonempty paragraph, including one in a quote or a list item
- **THEN** an editable empty paragraph appears above it in the same container
- **AND** the caret stays before the original text and the empty paragraph stays visible when the page is saved and reopened

### Requirement: Enter in lists, quotes and code
Enter SHALL split a list item at the caret and activate the second item. New empty task items SHALL be unchecked. The parts of a completed task that contain its text SHALL stay checked. Ordered items SHALL be written sequentially from the list's first number. Enter in an empty item SHALL end the list or outdent a nested item. A quote SHALL keep added paragraphs inside it until Enter is pressed in an empty one. Enter SHALL keep indentation when breaking a line in a code or source block.

#### Scenario: Next task item
- **WHEN** Enter is pressed at the end of `- [x] done`
- **THEN** a new item `- [ ] ` follows with the cursor after it

#### Scenario: Ending a list
- **WHEN** Enter is pressed in an empty bullet item at the end of a list
- **THEN** the empty item becomes an empty paragraph after the list

#### Scenario: Leaving a quote
- **WHEN** Enter is pressed twice at the end of the quote `> one` and `after` is typed
- **THEN** the source reads `> one`, a blank line and `after`

#### Scenario: Splitting a completed task
- **WHEN** Enter is pressed at the start of `- [x] Done`
- **THEN** an unchecked empty task appears before `- [x] Done` and the caret stays before `Done`
- **AND** splitting a completed task in the middle leaves both text parts checked

#### Scenario: Splitting an ordered item
- **WHEN** Enter is pressed at the start of the first item in a list starting with `3. First` followed by `3. Next`
- **THEN** the source reads `3. `, `4. First`, and `5. Next`, with the caret before `First`

#### Scenario: Enter at the start of a code block
- **WHEN** Enter is pressed before the first character in a fenced or indented code block
- **THEN** the code box shows a new blank first line and the caret stays before the original code
- **AND** the saved code keeps its fenced or indented form, with the leading blank line visible when reopened

#### Scenario: Enter inside front matter or display math
- **WHEN** Enter is pressed at the start of the first body line inside existing YAML front matter or a standalone display math block
- **THEN** a blank line is added inside the delimiters and the block keeps its kind
- **AND** the opening delimiter stays intact if Enter is pressed on its own line

#### Scenario: Enter in legacy front matter without a mapping
- **WHEN** Enter is pressed at the start of the first body line of a front matter block whose body is a YAML scalar
- **THEN** the blank line is added after the first body line and the page keeps the front matter block

### Requirement: Blank lines in edited lists
An edited list SHALL use one form for all its items: tight, with no blank lines between items or before blocks inside items, or loose, with one blank line in each place except where blank lines represent an empty paragraph. A nested list SHALL take its own form. The form SHALL be applied after an editing command and when the caret leaves a block whose text was edited. Lists no edit changed, and pages only opened or saved, SHALL keep their source.

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
An edited list SHALL be loose when an item contains an empty paragraph or when a block needs a preceding blank line to stay distinct, such as a paragraph, an image or math on its own lines, or a `---` rule. It SHALL otherwise be tight, including when code blocks, tables, quotes, headings or nested lists can remain distinct without blank lines.

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
