## MODIFIED Requirements

### Requirement: Block-level live preview
The editor SHALL show the document as a sequence of rendered blocks (see markdown-rendering). The block holding the cursor SHALL stay rendered, showing as Markdown only the inline elements touching the cursor; code blocks, math blocks, diagrams, front matter, HTML blocks, thematic breaks and `[toc]` SHALL show their whole source while they hold the cursor. Blocks are paragraphs, headings, single list items, those code-like blocks, tables and footnote definitions, also when they are inside a quote or a list item: every block of a quote, and every block of a list item after its first one, is a block of its own, shown and edited inside the quote's bar or the item's indentation.

#### Scenario: Leaving a paragraph
- **WHEN** the cursor is in the paragraph `some **bold** text` and moves to the next block
- **THEN** the paragraph shows `some bold text` with `bold` in bold and no asterisks

#### Scenario: Entering a list item
- **WHEN** a rendered task list item is clicked
- **THEN** the item stays rendered with its checkbox and without `- [ ]`, with the cursor in its text

#### Scenario: Cursor in a bold word
- **WHEN** the cursor is moved into `bold` in the paragraph `some **bold** text`
- **THEN** the paragraph shows `some **bold** text` with the asterisks dimmed and `some` and `text` rendered

#### Scenario: Cursor elsewhere in the paragraph
- **WHEN** the cursor is in `some` of the paragraph `some **bold** [link](x.md)`
- **THEN** `bold` is shown in bold and `link` as a link, without their Markdown

#### Scenario: Editing a code block
- **WHEN** the cursor is in a fenced code block
- **THEN** the block shows its source, fences included

#### Scenario: A list inside a quote
- **WHEN** the item `b` of the quote `> - a` / `> - b` is clicked
- **THEN** the item is edited with its bullet inside the quote's bar, without `>` and `-`, and the item `a` stays rendered

#### Scenario: A heading inside a quote
- **WHEN** the cursor is in the heading of the quote `> ## Plan`
- **THEN** `Plan` is shown at the size of a level-2 heading inside the quote's bar

### Requirement: Converting syntax as it is typed
A block marker typed at the start of a block's text (`#` to `######`, `-`, `*`, `+`, `1.` or `1)`, `>`, and `[ ]` or `[x]` in a list item) SHALL be shown as the typed text until the space after it is typed; with the space, the block SHALL become that element (a heading, a list item, a task item, a quote), edited in its rendered form without the marker. This applies inside quotes and list items too. A line that makes a code fence (with an optional info string), a thematic break (`---`, `***`, `___`), a setext heading underline, `[toc]`, an HTML block or an indented code block SHALL be shown as the typed text, leaving the rest of the page as it was, until Enter is pressed or the cursor leaves the block. Enter after a line consisting of ` ``` ` (with an optional language) or `$$` SHALL add the closing fence and put the cursor inside; Enter after `---` as the first line of the document SHALL start front matter, and elsewhere make a thematic break followed by a new paragraph; Enter after a table header row (`| a | b |`) SHALL create a table (see table-editing).

#### Scenario: Code fence
- **WHEN** ` ```rust ` is typed in an empty paragraph and Enter is pressed
- **THEN** the source reads ` ```rust `, an empty line with the cursor, and ` ``` `, and the block is a Rust code block

#### Scenario: Code fence before Enter
- **WHEN** ` ``` ` is typed in an empty paragraph followed by other paragraphs
- **THEN** ` ``` ` is shown as text in the paragraph being edited and the following paragraphs stay rendered as paragraphs

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

#### Scenario: Thematic break
- **WHEN** `---` is typed in an empty paragraph after other text and Enter is pressed
- **THEN** `---` is shown as text until Enter, and then a thematic break is shown with the cursor in a new paragraph after it

### Requirement: Enter in lists, quotes and code
In a list item, Enter SHALL start a new item of the same kind (the next number, an unchecked box for a task item); Enter in an empty item SHALL end the list, or outdent the item when it is nested. In a quote, Enter SHALL continue the quote with a new paragraph, and Enter in an empty new paragraph of a quote SHALL leave the quote; the same holds for the blocks of a list item, which Enter in an empty new paragraph leaves. Lists, quotes and paragraphs inside a quote or list item SHALL behave as at the top level, inside it. In code, math and front matter blocks Enter SHALL insert a line break keeping the indentation.

#### Scenario: Next task item
- **WHEN** Enter is pressed at the end of `- [x] done`
- **THEN** a new item `- [ ] ` follows with the cursor after it

#### Scenario: Ending a list
- **WHEN** Enter is pressed in an empty bullet item at the end of a list
- **THEN** the empty item becomes an empty paragraph after the list

#### Scenario: Leaving a quote
- **WHEN** Enter is pressed twice at the end of the quote `> one` and `after` is typed
- **THEN** the source reads `> one`, a blank line and `after`

#### Scenario: Next item in a quote
- **WHEN** Enter is pressed at the end of the item `a` of the quote `> - a`
- **THEN** the source reads `> - a` and `> - ` with the cursor after it

### Requirement: Backspace and Delete across blocks
Backspace at the start of the text of a heading, list item or quote SHALL remove its block marker (`# `, `- `, `1. `, `- [ ] `, `> `), making the block a paragraph. Backspace at the very start of a paragraph SHALL join it to the end of the previous paragraph, heading or list item, and otherwise move the cursor to the end of the previous block; in a quote or list item, the previous block is the one inside it, and Backspace at the start of the first block of a quote SHALL take that block out of the quote. Delete at the very end of a block SHALL join the next block's text in the same way.

#### Scenario: Removing a heading marker
- **WHEN** the cursor is at the start of the heading `Plan` (source `## Plan`) and Backspace is pressed
- **THEN** the block becomes the paragraph `Plan`

#### Scenario: Joining paragraphs
- **WHEN** the cursor is at the start of the paragraph `two` that follows the paragraph `one` and Backspace is pressed
- **THEN** the two paragraphs become one paragraph `onetwo` with the cursor between them

#### Scenario: Out of a quote
- **WHEN** the cursor is at the start of `one` in the quote `> one` / `>` / `> two` and Backspace is pressed
- **THEN** the source reads `one`, a blank line and `> two`
