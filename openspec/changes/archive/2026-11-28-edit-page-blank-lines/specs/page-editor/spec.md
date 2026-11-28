## ADDED Requirements

### Requirement: Displaying Markdown blank lines
The rendered page SHALL show one editable row for each extra Markdown blank line between blocks and for each blank line at the document edges. A required Markdown separator or loose-list separator SHALL be represented by block spacing. Blank lines inside a code, YAML, or math block SHALL belong to that block.

#### Scenario: Extra lines between paragraphs
- **WHEN** two paragraphs have three blank source lines between them
- **THEN** the page shows the paragraph separation and two editable blank rows

#### Scenario: Blank rows beside a code block and at the end of the page
- **WHEN** extra blank lines appear before a fenced code block or after the last block
- **THEN** each extra line is an editable row
- **AND** a newline terminating the final nonempty source line contributes no blank row

#### Scenario: Blank rows in and around containers
- **WHEN** extra blank lines occur in a quote, in a list item, or outside those containers
- **THEN** each row is displayed and editable within the container it belongs to, or at the page level if outside

#### Scenario: Opening and saving existing blank lines
- **WHEN** a page with blank lines is opened and saved without editing
- **THEN** its blank lines are preserved

### Requirement: Editing Markdown blank lines
The user SHALL be able to click or move the caret into each visible blank row. Enter in a source-backed blank row SHALL add one blank line in its container and move the caret to the new row. Each insertion SHALL be undoable. Text typed on a blank row SHALL become a paragraph with the separators its neighbors require.

#### Scenario: Repeated Enter and reopening
- **WHEN** the caret is on a displayed blank line and Enter is pressed twice
- **THEN** two blank lines are added, the caret moves to the last new row, and both lines remain visible after saving and reopening
- **AND** each Undo takes back one insertion and restores the caret

#### Scenario: Typing on a blank line
- **WHEN** text is typed into an editable blank row between paragraphs
- **THEN** it becomes a separate paragraph and the neighboring paragraphs remain distinct

## MODIFIED Requirements

### Requirement: Clicking places the cursor
Clicking SHALL place the cursor:
- on text, at the clicked position with the block rendered;
- in an empty list item, at the start of its text;
- to the right of a line within the column, at its end, including math and image lines;
- in an editable blank row, in that row;
- in block spacing, at the end of the block above or the start of the one below, whichever is closer;
- in a side margin, at the start of the line at the click's height.

#### Scenario: Click in the middle of a word
- **WHEN** the rendered paragraph `hello world` is clicked between `wor` and `ld`
- **THEN** the cursor is between `wor` and `ld` and the paragraph stays rendered

#### Scenario: Click beside an empty list item
- **WHEN** an empty list item `* ` is clicked far to the right of its bullet and `step` is typed
- **THEN** the item becomes `* step`

#### Scenario: Click right of a line with math
- **WHEN** the paragraph `Inline $x^2$ math` is clicked to the right of its text and `!` is typed
- **THEN** the paragraph becomes `Inline $x^2$ math!`

#### Scenario: Click in the gap between blocks
- **WHEN** the gap between the list item `two` and the paragraph `After` is clicked in its upper half and `!` is typed
- **THEN** the item becomes `two!`
- **WHEN** the same gap is clicked in its lower half and `?` is typed
- **THEN** the paragraph becomes `?After`

#### Scenario: Click in the margin beside a line
- **WHEN** the margin right of the paragraph `hello world` is clicked at the height of its line and `!` is typed
- **THEN** the paragraph becomes `!hello world`
- **WHEN** the margin left of a line of a paragraph of several lines is clicked and `?` is typed
- **THEN** `?` is inserted at the start of that line

### Requirement: Moving between blocks with the keyboard
Up and Down on the first and last line of a block, Left at its start and Right at its end SHALL move the cursor into the adjacent editable blank row or the previous or next block. Up/Down SHALL keep the cursor's horizontal position where the line is long enough.

#### Scenario: Down out of a block
- **WHEN** the cursor is on the last line of a paragraph and Down is pressed
- **THEN** the cursor is on the first line of the next block

### Requirement: Enter and Shift+Enter
At the start of a nonempty rendered paragraph or heading with a collapsed selection, Enter SHALL insert an empty paragraph above it and keep the caret with the text. Elsewhere in a paragraph, Enter SHALL split it at the caret and activate the second paragraph. Enter at the end of a top-level paragraph or heading SHALL create and activate a saved blank row below it. Shift+Enter SHALL break the line inside a paragraph.

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
- **THEN** the caret is in a new empty paragraph below the heading and it is visible after saving and reopening

#### Scenario: Enter at the start of a paragraph or quote
- **WHEN** Enter is pressed before the first character of a nonempty paragraph, including one in a quote or a list item
- **THEN** an editable empty paragraph appears above it in the same container
- **AND** the caret stays before the original text and the empty paragraph stays visible when the page is saved and reopened

### Requirement: Blank lines in edited lists
An edited list SHALL use one form for all its items: tight, with no blank lines between items or before blocks inside items, or loose, with one separator blank line in each place. It SHALL keep extra blank rows. A nested list SHALL take its own form. The form SHALL be applied after an editing command other than blank row insertion and deletion and when the caret leaves a block whose text was edited. Lists no edit changed, and pages only opened or saved, SHALL keep their source.

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

### Requirement: Deleting displayed empty paragraphs
With the caret in a displayed empty paragraph backed by Markdown source, Backspace and Delete SHALL remove exactly one Markdown blank line for that row, preserving the adjacent blocks and their required separators. Backspace SHALL prefer the preceding block end and Delete the following block start. When that block is absent, either key SHALL use the other block or a valid editing position in a page with only blank rows. Each deletion SHALL be one undoable edit.

#### Scenario: Empty paragraph between two paragraphs

- **WHEN** the source contains three blank lines between the paragraphs `one` and `two` and the user clicks the displayed empty paragraph and presses Delete
- **THEN** two blank source lines remain, the paragraphs remain distinct, and the caret is at the start of `two`
- **AND** Undo restores the deleted empty paragraph and its caret position

#### Scenario: Several empty paragraphs above a heading

- **WHEN** the user moves the caret from a heading into the closest of two empty paragraphs at the start of a page and presses Backspace
- **THEN** one empty paragraph remains and the caret is at the start of the heading text

#### Scenario: Empty paragraph in a container

- **WHEN** the caret is in a displayed empty paragraph inside a quote or list item and the user presses Backspace or Delete
- **THEN** the neighbouring blocks keep their quote markers or list indentation and remain in their original containers

#### Scenario: Deleting a trailing blank row or a row in a blank page
- **WHEN** Backspace or Delete is pressed on a displayed blank row at the end of the document or in a document consisting entirely of blank lines
- **THEN** that source line is removed and the caret stays at a valid editing position
