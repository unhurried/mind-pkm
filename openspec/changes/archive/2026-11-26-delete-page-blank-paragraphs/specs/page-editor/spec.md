## ADDED Requirements

### Requirement: Deleting displayed empty paragraphs

With the caret in a displayed empty paragraph backed by Markdown source, Backspace and Delete SHALL remove exactly that empty paragraph, preserving the adjacent blocks and their required separators. Backspace SHALL place the caret at the end of the preceding block and Delete at the start of the following block. At the start of the document, either key SHALL place the caret at the start of the following block. Each deletion SHALL be one undoable edit.

#### Scenario: Empty paragraph between two paragraphs

- **WHEN** the source contains three blank lines between the paragraphs `one` and `two` and the user clicks the displayed empty paragraph and presses Delete
- **THEN** one blank source line remains, the paragraphs remain distinct, and the caret is at the start of `two`
- **AND** Undo restores the deleted empty paragraph and its caret position

#### Scenario: Several empty paragraphs above a heading

- **WHEN** the user moves the caret from a heading into the closest of two empty paragraphs at the start of a page and presses Backspace
- **THEN** one empty paragraph remains and the caret is at the start of the heading text

#### Scenario: Empty paragraph in a container

- **WHEN** the caret is in a displayed empty paragraph inside a quote or list item and the user presses Backspace or Delete
- **THEN** the neighbouring blocks keep their quote markers or list indentation and remain in their original containers

## MODIFIED Requirements

### Requirement: Blank lines in edited lists
An edited list SHALL use one form for all its items: tight, with no blank lines between items or before blocks inside items, or loose, with one blank line in each place except where blank lines represent an empty paragraph. A nested list SHALL take its own form. The form SHALL be applied after an editing command other than empty paragraph deletion and when the caret leaves a block whose text was edited. Lists no edit changed, and pages only opened or saved, SHALL keep their source.

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
