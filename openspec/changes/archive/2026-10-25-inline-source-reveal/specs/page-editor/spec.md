## ADDED Requirements

### Requirement: Editing rendered text
Paragraphs, headings, list items, quotes and footnote definitions SHALL be edited in their rendered form, with a blinking text cursor and the selection drawn over the rendered text, rendered images and math included. Deleting part of a formatted text SHALL keep its formatting, and deleting all of it SHALL remove its Markdown markers too. Text typed at the cursor SHALL go where the source shown at the cursor puts it.

#### Scenario: Deleting the start of a bold word
- **WHEN** `bo` is selected in the rendered `**bold**` and deleted
- **THEN** the source reads `**ld**` and `ld` is shown in bold

#### Scenario: Deleting a whole bold word
- **WHEN** all of the bold `bold` in `a **bold** b` is selected and deleted
- **THEN** the source reads `a  b`

#### Scenario: Typing in Japanese
- **WHEN** Japanese text is composed with the IME in a paragraph being edited and committed
- **THEN** the composition is shown at the cursor and the committed text is inserted there

## MODIFIED Requirements

### Requirement: Block-level live preview
The editor SHALL show the document as a sequence of rendered blocks (see markdown-rendering). The block holding the cursor SHALL stay rendered, showing as Markdown only the inline elements touching the cursor; code blocks, math blocks, diagrams, front matter, HTML blocks, thematic breaks and `[toc]` SHALL show their whole source while they hold the cursor. Blocks are paragraphs, headings, single list items, quotes, those code-like blocks, tables and footnote definitions.

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

### Requirement: Styled source of the active block
Markdown shown while editing - the source of an inline element at the cursor, and the source of a code-like block - SHALL be styled like the rendering where text can show it: bold and italic text in bold and italic, code in the code font, links in the link color, and Markdown markers dimmed. Block markers (`#`, list markers and task boxes, `>`) SHALL NOT be shown while a block is edited: a heading keeps its size, a list item its bullet, number or checkbox, a quote its bar.

#### Scenario: Heading being edited
- **WHEN** the cursor is in the heading `## Plan`
- **THEN** `Plan` is shown at the size of a level-2 heading, without `##`

#### Scenario: Image at the cursor
- **WHEN** the cursor is moved next to the image `![map](Trip.assets/map.png)` in a paragraph
- **THEN** its source is shown dimmed in front of the rendered image

### Requirement: Clicking places the cursor
Clicking a rendered block SHALL put the cursor into it at the clicked position of its rendered text.

#### Scenario: Click in the middle of a word
- **WHEN** the rendered paragraph `hello world` is clicked between `wor` and `ld`
- **THEN** the cursor is between `wor` and `ld` and the paragraph stays rendered

### Requirement: Moving between blocks with the keyboard
Up and Down on the first and last line of a block, Left at its start and Right at its end SHALL move the cursor into the previous or next block. Up/Down SHALL keep the cursor's horizontal position where the line is long enough.

#### Scenario: Down out of a block
- **WHEN** the cursor is on the last line of a paragraph and Down is pressed
- **THEN** the cursor is on the first line of the next block

### Requirement: Backspace and Delete across blocks
Backspace at the start of the text of a heading, list item or quote SHALL remove its block marker (`# `, `- `, `1. `, `- [ ] `, `> `), making the block a paragraph. Backspace at the very start of a paragraph SHALL join it to the end of the previous paragraph, heading or list item, and otherwise move the cursor to the end of the previous block. Delete at the very end of a block SHALL join the next block's text in the same way.

#### Scenario: Removing a heading marker
- **WHEN** the cursor is at the start of the heading `Plan` (source `## Plan`) and Backspace is pressed
- **THEN** the block becomes the paragraph `Plan`

#### Scenario: Joining paragraphs
- **WHEN** the cursor is at the start of the paragraph `two` that follows the paragraph `one` and Backspace is pressed
- **THEN** the two paragraphs become one paragraph `onetwo` with the cursor between them
