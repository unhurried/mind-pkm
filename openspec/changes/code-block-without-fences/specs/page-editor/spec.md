## ADDED Requirements

### Requirement: Editing code blocks
A code block other than a diagram SHALL be edited as its code, in its box with its highlighting: its fences, the indentation of its lines and the prefixes of a quote or list item it is in are not shown, and lines typed get them. A field in the box SHALL show and change its language. Backspace in an empty code block SHALL turn it into an empty paragraph; Down at the last line of a code block ending the page SHALL start a new paragraph after it.

#### Scenario: Typing in a code block
- **WHEN** the cursor is at the end of `let x = 1;` in the quoted code block `> ```rust` / `> let x = 1;` / `> ```` and Enter and `x` are typed
- **THEN** the block shows `let x = 1;` and `x` without fences, and the source reads `> let x = 1;` and `> x` between the fences

#### Scenario: Changing the language
- **WHEN** `python` is entered in the language field of the code block ` ```rust `
- **THEN** its opening fence reads ` ```python ` and the code is highlighted as Python

#### Scenario: Removing an empty code block
- **WHEN** Backspace is pressed in a code block holding no code
- **THEN** the code block is gone and the cursor is in an empty paragraph in its place

## MODIFIED Requirements

### Requirement: Block-level live preview
The editor SHALL show the document as a sequence of rendered blocks (see markdown-rendering). The block holding the cursor SHALL stay rendered, showing as Markdown only the inline elements touching the cursor; math blocks, diagrams, front matter, HTML blocks, thematic breaks and `[toc]` SHALL show their whole source while they hold the cursor, and code blocks their code. Blocks are paragraphs, headings, single list items, quotes, those code-like blocks, tables and footnote definitions.

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
- **THEN** the block shows its code in its box, without the fences, and its language in a field
