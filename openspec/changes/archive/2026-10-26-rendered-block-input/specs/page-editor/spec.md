## ADDED Requirements

### Requirement: Blocks inside quotes and list items
Every block of a quote, and every block of a list item after its first one, SHALL be a block of its own, shown and edited like a block at the top level inside the quote's bar or the item's indentation: a list or heading inside a quote, and a quote, code block or paragraph inside a list item, are edited in their rendered form (or as source, for code-like blocks) without the quote's `>` or the item's indentation.

#### Scenario: A list inside a quote
- **WHEN** the item `b` of the quote `> - a` / `> - b` is clicked
- **THEN** the item is edited with its bullet inside the quote's bar, without `>` and `-`, and the item `a` stays rendered

#### Scenario: A heading inside a quote
- **WHEN** the cursor is in the heading of the quote `> ## Plan`
- **THEN** `Plan` is shown at the size of a level-2 heading inside the quote's bar

### Requirement: Line syntax completed by Enter
A line that makes a code fence (with an optional info string), a thematic break (`---`, `***`, `___`), a setext heading underline, `[toc]`, an HTML block or an indented code block SHALL be shown as the typed text, leaving the rest of the page as it was, until Enter is pressed or the cursor leaves the block. Enter after `---` SHALL start front matter on the first line of the document, and elsewhere make a thematic break followed by a new paragraph.

#### Scenario: Code fence before Enter
- **WHEN** ` ``` ` is typed in an empty paragraph followed by other paragraphs
- **THEN** ` ``` ` is shown as text in the paragraph being edited and the following paragraphs stay rendered as paragraphs

#### Scenario: Thematic break
- **WHEN** `---` is typed in an empty paragraph after other text and Enter is pressed
- **THEN** `---` is shown as text until Enter, and then a thematic break is shown with the cursor in a new paragraph after it

### Requirement: Editing inside quotes and list items
Inside a quote or list item, Enter, Shift+Enter, Tab, Backspace and Delete SHALL work as at the top level, keeping the quote's `>` and the item's indentation on the lines they change or add; the previous and next blocks are those inside it. Enter in an empty new paragraph inside a quote or list item SHALL leave it. Backspace at the start of the first block of a quote SHALL take that block out of the quote, and at the start of the first block of a list item after its text SHALL join that text.

#### Scenario: Next item in a quote
- **WHEN** Enter is pressed at the end of the item `a` of the quote `> - a`
- **THEN** the source reads `> - a` and `> - ` with the cursor after it

#### Scenario: Out of a quote
- **WHEN** the cursor is at the start of `one` in the quote `> one` / `>` / `> two` and Backspace is pressed
- **THEN** the source reads `one`, a blank line and `> two`

#### Scenario: A paragraph in a list item
- **WHEN** Enter is pressed at the end of the second paragraph of a list item and `x` is typed
- **THEN** `x` is a new paragraph of the same item, indented like it

## MODIFIED Requirements

### Requirement: Converting syntax as it is typed
A block marker typed at the start of a block's text (`#` to `######`, `-`, `*`, `+`, `1.`, `>`, a list item's `[ ]`), also inside quotes and list items, SHALL show as typed until the space after it; then the block SHALL become that element, edited rendered without the marker. Enter after a line ` ``` ` (with an optional language) or `$$` SHALL add the closing fence and put the cursor inside; Enter after a table header row (`| a | b |`) SHALL create a table (see table-editing).

#### Scenario: Code fence
- **WHEN** ` ```rust ` is typed in an empty paragraph and Enter is pressed
- **THEN** the source reads ` ```rust `, an empty line with the cursor, and ` ``` `, and the block is a Rust code block

#### Scenario: Heading as it is typed
- **WHEN** `# ` is typed at the start of an empty paragraph
- **THEN** the block is shown at the size of a level-1 heading while the title is typed

#### Scenario: A marker before its space
- **WHEN** `#` is typed in an empty paragraph
- **THEN** the paragraph shows `#`, and typing `tag` after it keeps the paragraph `#tag`

#### Scenario: A list as it is typed
- **WHEN** `*`, a space and `item` are typed at the start of an empty paragraph
- **THEN** `*` is shown until the space is typed, and then the block is a bullet list item showing `item` beside its bullet, without `*`

#### Scenario: A list in a quote as it is typed
- **WHEN** `> `, `- ` and `x` are typed in an empty paragraph
- **THEN** the block is a list item `x` with its bullet inside a quote's bar, edited rendered

### Requirement: Enter in lists, quotes and code
In a list item, Enter SHALL start a new item of the same kind (the next number, an unchecked box for a task item); Enter in an empty item SHALL end the list, or outdent the item when it is nested. In a quote, Enter SHALL continue the quote with a new paragraph, and Enter in that paragraph while it is empty SHALL leave the quote. In code, math and front matter blocks Enter SHALL insert a line break keeping the indentation.

#### Scenario: Next task item
- **WHEN** Enter is pressed at the end of `- [x] done`
- **THEN** a new item `- [ ] ` follows with the cursor after it

#### Scenario: Ending a list
- **WHEN** Enter is pressed in an empty bullet item at the end of a list
- **THEN** the empty item becomes an empty paragraph after the list

#### Scenario: Leaving a quote
- **WHEN** Enter is pressed twice at the end of the quote `> one` and `after` is typed
- **THEN** the source reads `> one`, a blank line and `after`
