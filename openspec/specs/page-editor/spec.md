# page-editor Specification

## Purpose
The built-in Markdown editor: windows that show and edit a Markdown file as a live preview, with the editing commands, view options and saving behavior of a WYSIWYG editor, kept in step with the PKM commands that touch the file.

## Requirements

### Requirement: Editor windows
Each Markdown file SHALL be edited in a window of its own, titled with the file name (`*` appended while it has unsaved changes) and ending with "Mind PKM". Opening a file that is already open SHALL bring its window to the front instead of opening another. A new editor window SHALL open with the size the last editor window was given.

#### Scenario: Opening the same page twice
- **WHEN** a page is opened in the editor and opened again from the map
- **THEN** the existing window comes to the front and no second window opens

#### Scenario: Title with unsaved changes
- **WHEN** `Plan.md` is edited and not saved
- **THEN** the window title is `Plan.md * - Mind PKM`

### Requirement: Files the editor opens
The editor SHALL open UTF-8 files, with or without a byte-order mark; a file that is not valid UTF-8 or cannot be read SHALL be reported (`failed to open <path>: <reason>`) and no window SHALL open.

#### Scenario: Not UTF-8
- **WHEN** a `.md` file encoded in Shift_JIS is opened
- **THEN** an error naming the file is shown and no editor window opens

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

### Requirement: Styled source of the active block
Markdown shown while editing - the source of an inline element at the cursor, and the source of a code-like block - SHALL be styled like the rendering where text can show it: bold and italic text in bold and italic, code in the code font, links in the link color, and Markdown markers dimmed. Block markers (`#`, list markers and task boxes, `>`) SHALL NOT be shown while a block is edited: a heading keeps its size, a list item its bullet, number or checkbox, a quote its bar.

#### Scenario: Heading being edited
- **WHEN** the cursor is in the heading `## Plan`
- **THEN** `Plan` is shown at the size of a level-2 heading, without `##`

#### Scenario: Image at the cursor
- **WHEN** the cursor is moved next to the image `![map](Trip.assets/map.png)` in a paragraph
- **THEN** its source is shown dimmed in front of the rendered image

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

### Requirement: Enter and Shift+Enter
Enter SHALL end the paragraph and start a new one after it (in a heading, a new paragraph after the heading). Shift+Enter SHALL break the line inside the paragraph.

#### Scenario: New paragraph
- **WHEN** the cursor is in the middle of `onetwo` (after `one`) and Enter is pressed
- **THEN** the source reads `one`, a blank line, `two`, with the cursor at the start of `two`

#### Scenario: Line break in a paragraph
- **WHEN** Shift+Enter is pressed after `one` in `onetwo`
- **THEN** the paragraph's source reads `one` and `two` on two lines and it is shown with a line break

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

### Requirement: Backspace and Delete across blocks
Backspace at the start of the text of a heading, list item or quote SHALL remove its block marker (`# `, `- `, `1. `, `- [ ] `, `> `), making the block a paragraph. Backspace at the very start of a paragraph SHALL join it to the end of the previous paragraph, heading or list item, and otherwise move the cursor to the end of the previous block. Delete at the very end of a block SHALL join the next block's text in the same way.

#### Scenario: Removing a heading marker
- **WHEN** the cursor is at the start of the heading `Plan` (source `## Plan`) and Backspace is pressed
- **THEN** the block becomes the paragraph `Plan`

#### Scenario: Joining paragraphs
- **WHEN** the cursor is at the start of the paragraph `two` that follows the paragraph `one` and Backspace is pressed
- **THEN** the two paragraphs become one paragraph `onetwo` with the cursor between them

### Requirement: Indenting list items
Tab and Shift+Tab in a list item SHALL indent the item under the previous item and outdent it, together with its nested items. In code and math blocks they SHALL indent and unindent the line.

#### Scenario: Nesting an item
- **WHEN** the second item of a bullet list has the cursor and Tab is pressed
- **THEN** it becomes a nested item of the first one

#### Scenario: Nesting an empty item
- **WHEN** Enter starts a new item below an item and Tab is pressed, at any depth, before anything is typed
- **THEN** the empty item stays a list item, shown without its marker as typed text, and what is typed next goes into it

### Requirement: Source mode
Ctrl+/ SHALL switch between the live preview and a source mode that shows the whole file as Markdown text with the same styling, and back, keeping the cursor at the same place in the text. Undo history SHALL be shared by both modes.

#### Scenario: Selecting across paragraphs
- **WHEN** Ctrl+/ is pressed and text spanning three paragraphs is selected and copied
- **THEN** the clipboard holds the Markdown of exactly the selected text

### Requirement: Selecting several blocks
Outside source mode, a selection reaching beyond a block SHALL select whole blocks: Shift+Up/Down at the first/last line of a block, Shift+click on another block, and Ctrl+A pressed a second time (the first selects the active block's text) SHALL select a range of blocks, shown highlighted. Copy, Cut, Delete and typing SHALL act on all selected blocks.

#### Scenario: Deleting three blocks
- **WHEN** three paragraphs are selected with Shift+Down and Delete is pressed
- **THEN** the three paragraphs are removed and the cursor is in the block that followed them

### Requirement: Paragraph shortcuts
Ctrl+1 to Ctrl+6 SHALL make the active block a heading of that level and Ctrl+0 a paragraph; Ctrl+= and Ctrl+- SHALL raise and lower the heading level (a paragraph becomes a level-6 heading with Ctrl+=, a level-6 heading a paragraph with Ctrl+-). Ctrl+Shift+K SHALL insert a code block, Ctrl+Shift+M a math block, Ctrl+T a table; Ctrl+Shift+Q SHALL toggle a quote, Ctrl+Shift+[ an ordered list and Ctrl+Shift+] a bullet list on the selected blocks or the active one.

#### Scenario: Heading by shortcut
- **WHEN** the cursor is in the paragraph `Plan` and Ctrl+2 is pressed
- **THEN** the block becomes `## Plan`

#### Scenario: Bullet list over several blocks
- **WHEN** two paragraphs are selected and Ctrl+Shift+] is pressed
- **THEN** both become items of one bullet list

### Requirement: Format shortcuts
Ctrl+B, Ctrl+I, Ctrl+U, Ctrl+Shift+` and Alt+Shift+5 SHALL toggle bold (`**`), italic (`*`), underline (`<u>`), inline code (`` ` ``) and strikethrough (`~~`) around the selection, or around the word at the cursor when nothing is selected; applying one where it already applies SHALL remove it. Ctrl+K SHALL insert a link `[text](url)` around the selection with the cursor in the URL; Ctrl+Shift+I SHALL insert images (see image-insertion).

#### Scenario: Bold and back
- **WHEN** `word` is selected and Ctrl+B is pressed twice
- **THEN** the text is `**word**` after the first press and `word` after the second

#### Scenario: Link from a selection
- **WHEN** `docs` is selected and Ctrl+K is pressed
- **THEN** the text becomes `[docs]()` with the cursor between the parentheses

### Requirement: Clear formatting
Ctrl+\ SHALL remove the inline formatting (bold, italic, underline, strikethrough, highlight, inline code, sub/superscript and inline HTML tags) from the selection, or from the active block when nothing is selected, keeping the text and the links.

#### Scenario: Clearing a sentence
- **WHEN** `a **b** _c_ <u>d</u>` is selected and Ctrl+\ is pressed
- **THEN** the text becomes `a b c d`

### Requirement: Auto-pairing
Typing `(`, `[`, `{`, `"`, `'`, `` ` ``, `（`, `「`, `『`, `【` SHALL insert the closing character after the cursor; in code and math blocks only `(`, `[`, `{`, `"`, `'` SHALL be paired. With text selected, `*`, `_`, `~`, `$` (and the opening characters) SHALL enclose the selection. Typing a closing character right before the same one SHALL move over it, and Backspace between an empty pair SHALL delete both. The Edit menu SHALL turn auto-pairing off and on, remembered across starts.

#### Scenario: Pairing a parenthesis
- **WHEN** `(` is typed and then `x` and `)`
- **THEN** the text is `(x)` with the cursor after `)`

#### Scenario: Enclosing a selection
- **WHEN** `word` is selected and `*` is typed
- **THEN** the text becomes `*word*`

### Requirement: Selection helpers
Ctrl+D SHALL select the word at the cursor; Ctrl+L SHALL select the line (in a table, the row); Ctrl+E SHALL select the formatted span the cursor is in (in a table, the cell).

#### Scenario: Selecting a styled span
- **WHEN** the cursor is inside `**bold text**` and Ctrl+E is pressed
- **THEN** `bold text` is selected

### Requirement: Undo and redo
Ctrl+Z SHALL undo and Ctrl+Y (or Ctrl+Shift+Z) SHALL redo the editor's changes, consecutive typing counting as one change, and SHALL put the cursor where the change was.

#### Scenario: Undoing typing
- **WHEN** `hello` is typed into a paragraph and Ctrl+Z is pressed
- **THEN** the paragraph is as before `hello` was typed

### Requirement: Find and replace
Ctrl+F SHALL open a find bar (filled with the selected text), Ctrl+H the same bar with a replace field. Matches SHALL be highlighted, case-insensitively unless the match-case option is on; Enter and F3 SHALL go to the next match and Shift+F3 to the previous one, selecting it in its block. Replace SHALL replace the current match and go to the next; Replace All SHALL replace every match as one change. Esc SHALL close the bar.

#### Scenario: Replace all
- **WHEN** the page contains `foo` three times and Replace All with `bar` is used
- **THEN** all three become `bar` and one Ctrl+Z restores them

### Requirement: Word and character counts
The status bar SHALL show the document's word and character count (Markdown markers excluded; each CJK character counts as a word), and the counts of the selection while text or blocks are selected.

#### Scenario: Counting a selection
- **WHEN** `two words` is selected
- **THEN** the status bar shows the selection's 2 words and 9 characters

### Requirement: Clipboard
Ctrl+C SHALL put the selection on the clipboard as HTML (rich text) together with its Markdown as plain text; Ctrl+Shift+C SHALL copy the Markdown only. Ctrl+V SHALL paste text as Markdown, an HTML clipboard from another application converted to Markdown (Smart Paste), image data and copied image files as inserted images (see image-insertion); Ctrl+Shift+V SHALL paste the plain text only.

#### Scenario: Smart Paste
- **WHEN** a web page's `<b>bold</b> and <a href="https://x.org">link</a>` is copied in a browser and pasted
- **THEN** `**bold** and [link](https://x.org)` is inserted

#### Scenario: Pasting what was copied here
- **WHEN** a selection is copied in the editor and pasted elsewhere in it
- **THEN** exactly the copied Markdown is inserted

### Requirement: Pasting into code
Inside a code block (and the other code-like blocks: math, front matter, HTML) Ctrl+V SHALL paste the clipboard's plain text as it is, without converting an HTML flavor to Markdown. A clipboard holding HTML only SHALL paste the text of that HTML, one line per block element.

#### Scenario: Pasting code into a code block
- **WHEN** code copied from VS Code (HTML and plain text) is pasted into a code block
- **THEN** the plain text is inserted without backslash escapes or extra blank lines

#### Scenario: Pasting HTML only into a code block
- **WHEN** a clipboard holding only `<div><span>let a_b = c*d;</span></div><div><span>x &lt; y</span></div>` is pasted into a code block
- **THEN** `let a_b = c*d;` and `x < y` are inserted on two lines

### Requirement: Links and task boxes in rendered blocks
Ctrl+click on a rendered link SHALL follow it: a relative `.md` file opens in the editor, `#anchor` scrolls to the heading with that anchor, another file or directory opens with the default application, a URL in the browser. Clicking the box of a rendered task item SHALL check or uncheck it as one undoable change.

#### Scenario: Link to another page
- **WHEN** `[see](Other.md)` is Ctrl+clicked
- **THEN** `Other.md` (next to the page) opens in an editor window

#### Scenario: Checking a task
- **WHEN** the box of `- [ ] call` is clicked
- **THEN** the source becomes `- [x] call` and the item shows a checked box

### Requirement: Outline
Ctrl+Shift+1 SHALL show and hide an outline beside the document listing the headings, indented by level; clicking one SHALL scroll to that heading.

#### Scenario: Jumping to a heading
- **WHEN** the outline is shown and `Journal` is clicked in it
- **THEN** the document scrolls so that the heading `Journal` is at the top

### Requirement: Full screen and zoom
F11 SHALL switch the editor window between full screen and its normal size. Ctrl+Shift+= and Ctrl+Shift+- SHALL zoom the document in and out, and Ctrl+Shift+0 SHALL reset it to 100%; the zoom level SHALL be shown in the status bar and remembered for new editor windows.

#### Scenario: Zooming in
- **WHEN** Ctrl+Shift+= is pressed in an editor at 100%
- **THEN** the document's text is shown larger and the status bar shows the new zoom level

### Requirement: Light and dark themes
The editor's View menu SHALL switch every editor window between a light and a dark theme, remembered across starts. The map window SHALL stay light.

#### Scenario: Dark theme
- **WHEN** the dark theme is chosen in one editor window
- **THEN** all editor windows show light text on a dark background

### Requirement: Editor menus
An editor window SHALL have a menu bar with File, Edit, Paragraph, Format, Table and View menus listing every editor command with its shortcut.

#### Scenario: Shortcut shown
- **WHEN** the Format menu is opened
- **THEN** Bold is listed with Ctrl+B

### Requirement: Saving pages
Ctrl+S SHALL write the page as UTF-8 with a byte-order mark, keeping the file's line endings (CRLF when the file used CRLF); nothing SHALL be written before. Only the text the user changed SHALL differ in the saved file.

#### Scenario: Saving a CRLF page
- **WHEN** a page with CRLF line endings and a BOM gets one word changed and is saved
- **THEN** the file differs from before only in that word, still with CRLF and the BOM

### Requirement: Unsaved changes in an editor
Closing an editor window with unsaved changes (also with Ctrl+W) SHALL ask whether to save, discard or cancel.

#### Scenario: Cancel closing
- **WHEN** an edited page's window is closed and Cancel is chosen
- **THEN** the window stays open with the changes

### Requirement: Dialogs of an editor are modal
While a dialog (a message, the question about unsaved changes, the question about a file changed on disk) is shown in an editor window, no key and no text from an input method SHALL change the page behind it. Enter SHALL choose the dialog's default button (OK, Save, Reload respectively) and Escape SHALL cancel it.

#### Scenario: Typing while the unsaved changes question is shown
- **WHEN** an edited page's window is closed with Ctrl+W and `LEAK` is typed while the question is shown
- **THEN** the page does not change and the question stays

#### Scenario: Enter saves
- **WHEN** an edited page's window is closed with Ctrl+W and Enter is pressed
- **THEN** the page is saved and the window closes

### Requirement: Files changed on disk
When an open file changes on disk, the editor SHALL reload it if it has no unsaved changes, keeping the cursor near where it was; with unsaved changes it SHALL ask whether to reload (losing them) or keep the editor's text.

#### Scenario: Changed by another application
- **WHEN** an unchanged open page is modified by another application and the editor window is activated
- **THEN** the editor shows the new text

### Requirement: PKM commands on open pages
Edit (F2) renaming a page or a directory, Add Assets and Delete (F11) SHALL first save the open editors of the affected files that have unsaved changes. Afterwards a renamed page's editor SHALL show the renamed file with its updated text, Add Assets' page SHALL be reloaded, and the editors of deleted files SHALL close. When a rename changed the text of an open page, Undo in its editor SHALL NOT go back across that change.

#### Scenario: Renaming an open page
- **WHEN** page `A` is open with unsaved changes and renamed to `B` with F2
- **THEN** the changes are saved, `B.md` holds them with `A.assets` references renamed, and the editor shows `B.md`

#### Scenario: Undo after renaming an open page
- **WHEN** page `A` with a reference `A.assets` is open, renamed to `B` with F2 and Undo is pressed in its editor
- **THEN** the page still references `B.assets`

#### Scenario: Deleting an open page
- **WHEN** page `P` is open in an editor and deleted with F11
- **THEN** its editor window closes without asking

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

### Requirement: Typing with no block active
When no block is active, typing a character, Up, Down, Enter, Home or End SHALL start editing at the cursor, in what the block there shows (in a heading, after its marker). A typed character SHALL be typed into the block; Home and End SHALL move to the start or end of the line.

#### Scenario: Typing right after opening a page
- **WHEN** the page `# Page1` is opened and `abc` is typed without clicking into it
- **THEN** the page reads `# abcPage1` and the heading stays a heading

#### Scenario: End right after opening a page
- **WHEN** the page `# Page1` is opened, End is pressed and ` unsaved` is typed
- **THEN** the page reads `# Page1 unsaved`
