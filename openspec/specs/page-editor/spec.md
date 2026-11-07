# page-editor Specification

## Purpose
The built-in Markdown editor: windows that show and edit a Markdown file as a live preview, with the editing commands, view options and saving behavior of a WYSIWYG editor, kept in step with the PKM commands that touch the file.

Terms used below: *blocks* are paragraphs, headings, single list items, code-like blocks, tables and footnote definitions, also inside quotes and list items (see Blocks inside quotes and list items). *Code-like blocks* are code blocks (Mermaid diagrams included), math blocks, front matter, HTML blocks, thematic breaks and `[toc]`. The *active block* is the block holding the cursor.

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

### Requirement: Opening files from an editor
File → Open... (Ctrl+Shift+O) in an editor window SHALL pick a `.md` file in a file dialog that starts in the folder of the window's file, and open it in an editor window of its own (bringing its window to the front when it is open already), in the built-in editor whatever Use Built-in Editor says. It SHALL work alike with and without a map window.

#### Scenario: Opening another file
- **WHEN** Ctrl+Shift+O is pressed in the editor of `Notes.md` and `Other.md` is chosen
- **THEN** `Other.md` opens in a new editor window and the window of `Notes.md` stays open

#### Scenario: A file that is open already
- **WHEN** `Other.md` is open in an editor window, Ctrl+Shift+O is pressed in the editor of `Notes.md` and `Other.md` is chosen
- **THEN** the window of `Other.md` comes to the front and no second window opens

#### Scenario: Cancelling the dialog
- **WHEN** Ctrl+Shift+O is pressed in an editor window and the file dialog is cancelled
- **THEN** nothing opens and the page is unchanged

### Requirement: Errors of opening from an editor
When a file cannot be opened from an editor window - chosen with File → Open... or a link that is followed - the error SHALL be shown in that editor window, not in the map window.

#### Scenario: A file that is not UTF-8
- **WHEN** Ctrl+Shift+O is pressed in the editor of `Notes.md` and a `.md` file encoded in Shift_JIS is chosen
- **THEN** the editor of `Notes.md` shows an error naming the file and no editor window opens

#### Scenario: A link to a missing page
- **WHEN** `[see](Gone.md)` is Ctrl+clicked and `Gone.md` does not exist
- **THEN** the editor window holding the link shows an error naming `Gone.md`

### Requirement: Saving pages
The editor SHALL write a page only when it is saved - by Ctrl+S, by Save in a question about unsaved changes, or before a PKM command that touches it (see PKM commands on open pages) - never on its own. It SHALL write the page as UTF-8 with a byte-order mark, keeping the file's line endings (CRLF when the file used CRLF), so that only the text the user changed differs in the saved file.

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

### Requirement: Block-level live preview
The editor SHALL show the document as a sequence of rendered blocks (see markdown-rendering). The active block SHALL stay rendered, showing as Markdown only the inline elements touching the cursor; a code block SHALL show its code (see Editing code blocks) and the other code-like blocks their whole source while active.

#### Scenario: Leaving a paragraph
- **WHEN** the cursor is in the paragraph `some **bold** text` and moves to the next block
- **THEN** the paragraph shows `some bold text` with `bold` in bold and no asterisks

#### Scenario: Cursor in a bold word
- **WHEN** the cursor is moved into `bold` in the paragraph `some **bold** text`
- **THEN** the paragraph shows `some **bold** text` with the asterisks dimmed and `some` and `text` rendered

#### Scenario: Cursor elsewhere in the paragraph
- **WHEN** the cursor is in `some` of the paragraph `some **bold** [link](x.md)`
- **THEN** `bold` is shown in bold and `link` as a link, without their Markdown

#### Scenario: Entering a list item
- **WHEN** a rendered task list item is clicked
- **THEN** the item stays rendered with its checkbox and without `- [ ]`, with the cursor in its text

### Requirement: Styled source of the active block
Markdown shown in the active block - the source of an inline element at the cursor, and the source of a code-like block - SHALL be styled like the rendering where text can show it: bold and italic text in bold and italic, code in the code font, links in the link color, and Markdown markers dimmed. Block markers (`#`, list markers and task boxes, `>`) SHALL NOT be shown in the active block: a heading keeps its size, a list item its bullet, number or checkbox, a quote its bar.

#### Scenario: Heading being edited
- **WHEN** the cursor is in the heading `## Plan`
- **THEN** `Plan` is shown at the size of a level-2 heading, without `##`

#### Scenario: Image at the cursor
- **WHEN** the cursor is moved next to the image `![map](Trip.assets/map.png)` in a paragraph
- **THEN** its source is shown dimmed in front of the rendered image

### Requirement: Stable line height
A line of text SHALL be as tall while its block is active as when it is not, whatever fonts and sizes its inline elements mix, so that moving the cursor never shifts the blocks below.

#### Scenario: Inline code in a list item
- **WHEN** the cursor moves into or out of a list item containing `` `code` `` between plain text
- **THEN** the item, and so the items below it, keep their height

### Requirement: Blocks inside quotes and list items
Every block of a quote, and every block of a list item after its first one, SHALL be a block of its own, shown and edited like a block at the top level inside the quote's bar or the item's indentation: a list or heading inside a quote, and a quote, code block or paragraph inside a list item, are edited in their rendered form (or as source, for code-like blocks) without the quote's `>` or the item's indentation.

#### Scenario: A list inside a quote
- **WHEN** the item `b` of the quote `> - a` / `> - b` is clicked
- **THEN** the item is edited with its bullet inside the quote's bar, without `>` and `-`, and the item `a` stays rendered

#### Scenario: A heading inside a quote
- **WHEN** the cursor is in the heading of the quote `> ## Plan`
- **THEN** `Plan` is shown at the size of a level-2 heading inside the quote's bar

### Requirement: Cursor in a block's hidden syntax
A cursor placed in a block edited rendered, in syntax the block does not show (a heading's `#`s, a list item's marker, a quote's `>`), SHALL be where the block shows it: at the start of its text. This holds however the cursor got there - an arrow key from a neighbouring block, a press beside the text, leaving selected blocks - so that typed text goes into the block's text.

#### Scenario: Moving into a heading
- **WHEN** the cursor is at the end of the paragraph before the heading `### aaa`, Right is pressed and `bbb ` is typed
- **THEN** the heading reads `### bbb aaa`

### Requirement: Clicking places the cursor
A click on the page SHALL always leave a cursor in a block:
- on a block's text: at the clicked position, the block staying rendered;
- anywhere in an empty list item's row: at the start of its text;
- right of a line, within the page's column: at the end of that line, also with math or an image;
- between two blocks: in the nearer one, at the end of the block above or the start of the block below;
- in the margin left or right of the column: at the start of the line at the click's height.

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

### Requirement: Typing with no block active
When no block is active (as right after a page opens), typing a character, Up, Down, Enter, Home or End SHALL start editing at the cursor, in what the block there shows (in a heading, after its marker). A typed character SHALL be typed into the block; Home and End SHALL move to the start or end of the line.

#### Scenario: Typing right after opening a page
- **WHEN** the page `# Page1` is opened and `abc` is typed without clicking into it
- **THEN** the page reads `# abcPage1` and the heading stays a heading

#### Scenario: End right after opening a page
- **WHEN** the page `# Page1` is opened, End is pressed and ` unsaved` is typed
- **THEN** the page reads `# Page1 unsaved`

### Requirement: Moving between blocks with the keyboard
Up and Down on the first and last line of a block, Left at its start and Right at its end SHALL move the cursor into the previous or next block. Up/Down SHALL keep the cursor's horizontal position where the line is long enough.

#### Scenario: Down out of a block
- **WHEN** the cursor is on the last line of a paragraph and Down is pressed
- **THEN** the cursor is on the first line of the next block

### Requirement: Scrolling and the cursor
The page SHALL scroll to keep the cursor in view when the cursor moves, the selection changes or the text is edited. Otherwise the mouse wheel and the scroll bar SHALL scroll the page freely, also taking the cursor out of view.

#### Scenario: Reading on past the cursor
- **WHEN** the cursor is in a paragraph near the top of a long page and the page is scrolled down with the mouse wheel
- **THEN** the page keeps scrolling, past the cursor's line

#### Scenario: Typing after scrolling away
- **WHEN** the cursor's line has been scrolled out of view and a character is typed
- **THEN** the page scrolls back to show the cursor

### Requirement: Selecting several blocks
Outside source mode, a selection reaching beyond a block SHALL select whole blocks: Shift+Up/Down at the first/last line of a block, Shift+click on another block, dragging the mouse from a block onto another, and Ctrl+A pressed a second time (the first selects the active block's text) SHALL select a range of blocks, shown highlighted. Copy, Cut, Delete and typing SHALL act on all selected blocks.

#### Scenario: Deleting three blocks
- **WHEN** three paragraphs are selected with Shift+Down and Delete is pressed
- **THEN** the three paragraphs are removed and the cursor is in the block that followed them

#### Scenario: Selected block with a background of its own
- **WHEN** a level-2 heading (a light band), a code block or a table is among the selected blocks
- **THEN** the highlight is drawn over the block's own background as well, so the block is as visibly selected as a paragraph

#### Scenario: Selected blocks are one highlighted band
- **WHEN** a paragraph, a list item and a paragraph inside a quote are selected together
- **THEN** the highlight spans the page's width, a list item's indentation and a quote's bar included, and fills the space between the selected blocks, so the selection reads as one uninterrupted band

#### Scenario: Dragging over blocks
- **WHEN** the mouse button is pressed in the text of the second of four paragraphs and the pointer is dragged onto the fourth
- **THEN** the second to fourth paragraphs are selected, and moving the pointer back onto the third leaves the second and third selected until the button is released

#### Scenario: Dragging within a block that is not the active one
- **WHEN** the mouse button is pressed in the rendered text of a paragraph other than the active one and the pointer is dragged along the text with the button held
- **THEN** the paragraph becomes the active block and the text between the pressed point and the pointer is selected, as when dragging in the active block

#### Scenario: Dragging within a code block or a table that is not the active one
- **WHEN** the mouse button is pressed in a rendered code block or math block other than the active one and the pointer is dragged along its source with the button held
- **THEN** the block becomes the active block and the text between the pressed point and the pointer is selected
- **WHEN** the button is pressed in a cell of a rendered table and the pointer is dragged onto other cells
- **THEN** the table becomes the active block and the cells between are selected

### Requirement: Selection helpers
Edit → Select Word SHALL select the word at the cursor (it has no shortcut; a double click selects a word too); Ctrl+L SHALL select the line (in a table, the row); Ctrl+E SHALL select the formatted span the cursor is in (in a table, the cell).

#### Scenario: Selecting a styled span
- **WHEN** the cursor is inside `**bold text**` and Ctrl+E is pressed
- **THEN** `bold text` is selected

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

### Requirement: Block markers converted by a space
A block marker typed at the start of a block's text (`#` to `######`, `-`, `*`, `+`, `1.`, `>`, a list item's `[ ]`), also inside quotes and list items, SHALL show as typed until the space after it; then the block SHALL become that element, edited rendered without the marker.

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

### Requirement: Line syntax completed by Enter
A line making a code fence (with an optional info string), a thematic break (`---`, `***`, `___`), a setext heading underline, `[toc]`, an HTML block or an indented code block SHALL show as typed, leaving the rest of the page as it was, until Enter or the cursor leaves the block. Enter after a code fence or `$$` SHALL add the closing fence with the cursor inside; after `---`, start front matter on the document's first line and elsewhere make a thematic break and a new paragraph.

#### Scenario: Code fence before Enter
- **WHEN** ` ``` ` is typed in an empty paragraph followed by other paragraphs
- **THEN** ` ``` ` is shown as text in the paragraph being edited and the following paragraphs stay rendered as paragraphs

#### Scenario: Code fence
- **WHEN** ` ```rust ` is typed in an empty paragraph and Enter is pressed
- **THEN** the source reads ` ```rust `, an empty line with the cursor, and ` ``` `, and the block is a Rust code block

#### Scenario: Thematic break
- **WHEN** `---` is typed in an empty paragraph after other text and Enter is pressed
- **THEN** `---` is shown as text until Enter, and then a thematic break is shown with the cursor in a new paragraph after it

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

### Requirement: Enter at the end of a list item with blocks below
Enter at the end of the text of a list item that has blocks below its text SHALL decide by the first of them:
- a nested item: the new item SHALL follow the text right away and the blocks below SHALL become the new item's, at their level; a blank line between the text and them SHALL go before the new item;
- another block (a paragraph, a code block, a quote): the blocks SHALL stay the item's and the new item SHALL follow them.

#### Scenario: New item above a nested item
- **WHEN** Enter is pressed at the end of `a` in the list item `a` with the nested item `b`
- **THEN** a new empty item follows `a` at its level, with the cursor in it, and `b` stays nested below the new item

#### Scenario: Nested item after a blank line
- **WHEN** Enter is pressed at the end of `a` in the list item `a` whose nested item `b` follows after a blank line
- **THEN** the blank line comes before the new empty item, and `b` stays nested right below it

#### Scenario: An item with a code block below its text
- **WHEN** Enter is pressed at the end of the item `* aaa` that has a code block below its text, and `b` is typed
- **THEN** the code block stays in the item `aaa`, and a new item `b` follows it

### Requirement: Backspace and Delete across blocks
Backspace at the start of the text of a heading, quote or the first list item at the top level SHALL remove its block marker (`# `, `- `, `1. `, `> `), making the block a paragraph. Backspace at the very start of a paragraph SHALL join it to the end of the previous paragraph, heading or list item, and otherwise move the cursor to the end of the previous block. Delete at the very end of a block SHALL join the next block's text in the same way.

#### Scenario: Removing a heading marker
- **WHEN** the cursor is at the start of the heading `Plan` (source `## Plan`) and Backspace is pressed
- **THEN** the block becomes the paragraph `Plan`

#### Scenario: Joining paragraphs
- **WHEN** the cursor is at the start of the paragraph `two` that follows the paragraph `one` and Backspace is pressed
- **THEN** the two paragraphs become one paragraph `onetwo` with the cursor between them

### Requirement: Blocks under a list item
Backspace at the start of the text of a list item that follows another item of its list, or that is the first item of a nested list, SHALL make it a block of that item (the previous item, or the item its list is nested in): an empty item SHALL become a new empty paragraph inside that item, where any block can be typed, and an item with text a paragraph of that item.

#### Scenario: A table under a list item
- **WHEN** Enter is pressed at the end of the list item `aaa`, then Backspace, then `| bbb | ccc |` is typed and Enter is pressed
- **THEN** the table with the header `bbb`, `ccc` is a block of the item `aaa`, shown below its text at its indentation, with the cursor in its first empty cell, and no blank line is written before the table

#### Scenario: An item into the item before
- **WHEN** the cursor is at the start of `b` in the list `- a` / `- b` / `- c` and Backspace is pressed
- **THEN** `b` is a second paragraph of the item `a`, `c` stays the next item of the list, and the list is written loose: one blank line before `b` and one before `- c`

### Requirement: Backspace in a task item
Backspace at the start of the text of a task item with text SHALL remove only its box, leaving a plain list item. Backspace at the start of an empty task item SHALL remove its box and its bullet with one press, doing what Backspace does at the start of an empty plain item (see Blocks under a list item, and Backspace and Delete across blocks): no empty item without its box SHALL be left in between.

#### Scenario: A task item with text
- **WHEN** the cursor is at the start of `call` in the task item `- [ ] call` and Backspace is pressed
- **THEN** the item becomes the plain list item `- call`

#### Scenario: An empty task item
- **WHEN** Enter is pressed at the end of the task item `- [ ] a`, which starts the empty task item `- [ ] `, and Backspace is pressed
- **THEN** the empty task item is gone and the cursor is in a new empty paragraph inside the item `a`, where `more` typed becomes its second paragraph

#### Scenario: An empty task item at the top of a page
- **WHEN** the page holds only the empty task item `- [ ] ` and Backspace is pressed at its start
- **THEN** the page holds an empty paragraph

### Requirement: Blank lines in edited lists
A list changed by an edit SHALL be written in one form for all of its items: tight, with no blank line between its items nor before the blocks inside its items, or loose, with exactly one blank line in each of these places. A nested list SHALL take its own form. The form SHALL be applied after an editing command and when the cursor leaves a block whose text was edited; lists no edit changed, and pages only opened or saved, SHALL keep their source.

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

### Requirement: Loose only where blank lines are needed
An edited list SHALL be written loose only when a block inside one of its items would not stay that block without a blank line before it - a paragraph, an image or math on lines of its own, a `---` rule - and tight otherwise: code blocks, tables, quotes, headings and nested lists inside items SHALL keep a list tight. A list SHALL become tight again when its last such block goes.

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

### Requirement: Blocks inserted in a list item
Inserting a code block, a math block, a horizontal rule or a table (also pasting cells as a table) while a list item's text is the active block SHALL put the new block into that item, right below its text and before the item's other blocks, at the item's indentation; the list SHALL stay one list.

#### Scenario: A code block inserted in an item
- **WHEN** the cursor is in the item `b` of the list `* a` / `* b` / `* c` and Ctrl+Shift+K is pressed
- **THEN** an empty code block is a block of the item `b`, the cursor is in it, `* c` is still an item of the same list, and the list stays tight

#### Scenario: An ordered list stays numbered
- **WHEN** the cursor is in the item `2.` of the list `1.` / `2.` / `3.` and Ctrl+T is pressed
- **THEN** the table is a block of the item `2.`, and the items keep the numbers 1, 2 and 3 of one list

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

### Requirement: Indenting list items
Tab and Shift+Tab in a list item SHALL indent the item under the previous item and outdent it, together with its nested items. In code and math blocks they SHALL indent and unindent the line.

#### Scenario: Nesting an item
- **WHEN** the second item of a bullet list has the cursor and Tab is pressed
- **THEN** it becomes a nested item of the first one

#### Scenario: Nesting an empty item
- **WHEN** Enter starts a new item below an item and Tab is pressed, at any depth, before anything is typed
- **THEN** the empty item stays a list item, shown without its marker as typed text, and what is typed next goes into it

### Requirement: Editing code blocks
A code block other than a diagram SHALL be edited as its code, in its box with its highlighting: its fences, the indentation of its lines and the prefixes of a quote or list item it is in are not shown, and lines typed get them. A field in the box SHALL show and change its language. Backspace in an empty code block SHALL turn it into an empty paragraph.

#### Scenario: Editing a code block
- **WHEN** the cursor is in a fenced code block
- **THEN** the block shows its code in its box, without the fences, and its language in a field

#### Scenario: Typing in a code block
- **WHEN** the cursor is at the end of `let x = 1;` in the quoted code block `> ```rust` / `> let x = 1;` / `> ```` and Enter and `x` are typed
- **THEN** the block shows `let x = 1;` and `x` without fences, and the source reads `> let x = 1;` and `> x` between the fences

#### Scenario: Changing the language
- **WHEN** `python` is entered in the language field of the code block ` ```rust `
- **THEN** its opening fence reads ` ```python ` and the code is highlighted as Python

#### Scenario: Removing an empty code block
- **WHEN** Backspace is pressed in a code block holding no code
- **THEN** the code block is gone and the cursor is in an empty paragraph in its place

### Requirement: Down at the end of a code block
Down at the last line of a code block SHALL start a new empty paragraph after it when the block ends the page, or when it is the last block of a list item or quote, inside that item or quote, where any block can be typed. Down in that paragraph while it is empty SHALL move on to the next block, leaving the page unchanged.

#### Scenario: A block after code ending a list item
- **WHEN** Down is pressed at the last line of a code block that is the last block of a list item followed by a paragraph outside the list, and `z` is typed
- **THEN** `z` is a new paragraph of the list item, after the code block, indented like it

#### Scenario: Moving on past the item
- **WHEN** Down is pressed twice at the last line of a code block that is the last block of a list item followed by the paragraph `next`
- **THEN** the cursor is in `next` and the page is unchanged

### Requirement: Auto-pairing
Typing `(`, `[`, `{`, `"`, `'`, `` ` ``, `（`, `「`, `『`, `【` SHALL insert the closing character after the cursor; in code and math blocks only `(`, `[`, `{`, `"`, `'` SHALL be paired. With text selected, `*`, `_`, `~`, `$` (and the opening characters) SHALL enclose the selection. Typing a closing character right before the same one SHALL move over it, and Backspace between an empty pair SHALL delete both. The Edit menu SHALL turn auto-pairing off and on, remembered across starts.

#### Scenario: Pairing a parenthesis
- **WHEN** `(` is typed and then `x` and `)`
- **THEN** the text is `(x)` with the cursor after `)`

#### Scenario: Enclosing a selection
- **WHEN** `word` is selected and `*` is typed
- **THEN** the text becomes `*word*`

### Requirement: Undo and redo
Ctrl+Z SHALL undo and Ctrl+Y (or Ctrl+Shift+Z) SHALL redo the editor's changes, consecutive typing counting as one change, and SHALL put the cursor where the change was. The undo history SHALL be shared by the live preview and source mode.

#### Scenario: Undoing typing
- **WHEN** `hello` is typed into a paragraph and Ctrl+Z is pressed
- **THEN** the paragraph is as before `hello` was typed

### Requirement: Paragraph shortcuts
Ctrl+1 to Ctrl+6 SHALL make the active block a heading of that level and Ctrl+0 a paragraph; Ctrl+= and Ctrl+- SHALL raise and lower the heading level (a paragraph becomes a level-6 heading with Ctrl+=, a level-6 heading a paragraph with Ctrl+-). Ctrl+Shift+K SHALL insert a code block, Ctrl+Shift+M a math block, Ctrl+T a table (see table-editing); Ctrl+Shift+Q SHALL toggle a quote, Ctrl+Shift+[ an ordered list and Ctrl+Shift+] a bullet list on the selected blocks or the active one.

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

### Requirement: Clipboard
Ctrl+C SHALL put the selection on the clipboard as HTML (rich text) together with its Markdown as plain text; Ctrl+Shift+C SHALL copy the Markdown only. Ctrl+V SHALL paste text as Markdown, an HTML clipboard from another application converted to Markdown (Smart Paste), image data and copied image files as inserted images (see image-insertion); Ctrl+Shift+V SHALL paste the plain text only.

#### Scenario: Smart Paste
- **WHEN** a web page's `<b>bold</b> and <a href="https://x.org">link</a>` is copied in a browser and pasted
- **THEN** `**bold** and [link](https://x.org)` is inserted

#### Scenario: Pasting what was copied here
- **WHEN** a selection is copied in the editor and pasted elsewhere in it
- **THEN** exactly the copied Markdown is inserted

### Requirement: Pasting into code
Inside a code block, math block, front matter or HTML block, Ctrl+V SHALL paste the clipboard's plain text as it is, without converting an HTML flavor to Markdown. A clipboard holding HTML only SHALL paste the text of that HTML, one line per block element.

#### Scenario: Pasting code into a code block
- **WHEN** code copied from VS Code (HTML and plain text) is pasted into a code block
- **THEN** the plain text is inserted without backslash escapes or extra blank lines

#### Scenario: Pasting HTML only into a code block
- **WHEN** a clipboard holding only `<div><span>let a_b = c*d;</span></div><div><span>x &lt; y</span></div>` is pasted into a code block
- **THEN** `let a_b = c*d;` and `x < y` are inserted on two lines

### Requirement: Find and replace
Ctrl+F SHALL open a find bar (filled with the selected text), Ctrl+H the same bar with a replace field. Matches SHALL be highlighted, case-insensitively unless the match-case option is on; Enter and F3 SHALL go to the next match and Shift+F3 to the previous one, selecting it in its block. Replace SHALL replace the current match and go to the next; Replace All SHALL replace every match as one change. Esc SHALL close the bar.

#### Scenario: Replace all
- **WHEN** the page contains `foo` three times and Replace All with `bar` is used
- **THEN** all three become `bar` and one Ctrl+Z restores them

### Requirement: Links and task boxes in rendered blocks
Ctrl+click on a rendered link SHALL follow it: a relative `.md` file opens in the editor, `#anchor` scrolls to the heading with that anchor, another file or directory opens with the default application, a URL in the browser. Clicking the box of a rendered task item SHALL check or uncheck it as one undoable change.

#### Scenario: Link to another page
- **WHEN** `[see](Other.md)` is Ctrl+clicked
- **THEN** `Other.md` (next to the page) opens in an editor window

#### Scenario: Checking a task
- **WHEN** the box of `- [ ] call` is clicked
- **THEN** the source becomes `- [x] call` and the item shows a checked box

### Requirement: Word and character counts
The status bar SHALL show the document's word and character count (Markdown markers excluded; each CJK character counts as a word), and the counts of the selection while text or blocks are selected.

#### Scenario: Counting a selection
- **WHEN** `two words` is selected
- **THEN** the status bar shows the selection's 2 words and 9 characters

### Requirement: Source mode
Ctrl+/ SHALL switch between the live preview and a source mode that shows the whole file as Markdown text with the same styling, and back, keeping the cursor at the same place in the text.

#### Scenario: Selecting across paragraphs
- **WHEN** Ctrl+/ is pressed and text spanning three paragraphs is selected and copied
- **THEN** the clipboard holds the Markdown of exactly the selected text

### Requirement: Outline
Ctrl+O SHALL show and hide an outline beside the document listing the headings, indented by level; clicking one SHALL scroll to that heading. Each editor window SHALL show or hide its own outline; a window that opens SHALL show the outline if it was last shown in any editor window and hide it if it was last hidden, remembered across starts.

#### Scenario: Jumping to a heading
- **WHEN** the outline is shown and `Journal` is clicked in it
- **THEN** the document scrolls so that the heading `Journal` is at the top

#### Scenario: The next window shows the outline
- **WHEN** Ctrl+O shows the outline in the editor of `Trip.md` and `Plan.md` is opened
- **THEN** the editor of `Plan.md` opens with the outline shown

#### Scenario: The next window hides the outline
- **WHEN** the outline is shown in the editors of `Trip.md` and `Plan.md`, Ctrl+O hides it in the editor of `Plan.md` and `Notes.md` is opened
- **THEN** the editor of `Notes.md` opens without the outline and the editor of `Trip.md` keeps it

#### Scenario: Remembered across starts
- **WHEN** the outline was last shown, the application is closed and started again and a page is opened
- **THEN** the page's editor opens with the outline shown

### Requirement: Folding the outline
An outline entry followed by headings of a lower level (up to the next heading of the same or a higher level) SHALL have an arrow that folds and unfolds the entries under it, without scrolling the document. By default the outline SHALL be unfolded down to H3: entries of H1 and H2 unfolded, those of H3 and below folded. What was folded or unfolded SHALL be kept while the page is edited, as long as the heading's text stays the same.

#### Scenario: Unfolded down to H3 by default
- **WHEN** the outline is shown for a page with `# A`, `## B`, `### C` and `#### D`
- **THEN** the outline lists `A`, `B` and `C`, with `C` folded and `D` hidden under it

#### Scenario: Unfolding an entry
- **WHEN** the arrow of the folded entry `C` is clicked
- **THEN** `D` is listed under `C` and the document does not scroll

#### Scenario: Folding an entry
- **WHEN** the arrow of the unfolded entry `A` is clicked
- **THEN** only `A` is listed, the headings under it hidden

#### Scenario: Editing the page
- **WHEN** `C` was unfolded and text is typed into a paragraph of the page
- **THEN** `C` stays unfolded

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

### Requirement: Page context menu
Right-clicking the page SHALL open a menu (see application-shell, Context menus) offering Cut, Copy, Copy as Markdown, Paste, Paste as Plain Text, Select All, Bold, Italic, Underline, Code, Strikethrough, Link and Clear Formatting, in rendered blocks, in the active block and in source mode alike. A right-click inside the selection SHALL keep it; a right-click anywhere else on the page SHALL first put the cursor where a click there would, without starting to select.

#### Scenario: Formatting the selection
- **WHEN** `world` is selected in the paragraph `hello world`, the selection is right-clicked and Bold is chosen
- **THEN** the paragraph becomes `hello **world**`

#### Scenario: Right-click outside the selection
- **WHEN** text in one paragraph is selected and another rendered paragraph is right-clicked
- **THEN** the cursor is in the right-clicked paragraph where it was clicked, nothing is selected, and the menu is open

### Requirement: A click ends with its release
A click on the page SHALL end with the release of the button, however soon the release follows the press: moving the pointer afterwards SHALL NOT select text, and a block that is edited as its source later (a paragraph that typing turns into a math block) SHALL keep the cursor where it is rather than where the block was clicked.

#### Scenario: Pointer moved after a quick click
- **WHEN** the paragraph `see link end` (with `link` a link) is clicked quickly, Home is pressed and the pointer is moved over `link`, which is Ctrl+clicked
- **THEN** no text is selected

#### Scenario: A paragraph completed into a math block
- **WHEN** the paragraph `$$` / `\frac{1}{0` / `$$` is clicked quickly before `rac`, End is pressed on its second line and `}X` is typed
- **THEN** the source is `$$` / `\frac{1}{0}X` / `$$`

### Requirement: Position kept as the page is laid out anew
When blocks above the visible part of the page change height without being edited - the page zoomed or its window resized, math or a diagram rendered, a block's height measured - the page SHALL scroll with them, so that the block at the top of the visible part stays where it is; after zooming or resizing, as far into that block as before for its new height. Opening, zooming or resizing a long page SHALL NOT wait for every block of the page to be laid out.

#### Scenario: Zooming a scrolled page
- **WHEN** a long page is scrolled down so that the heading `Section 30` is at the top and Ctrl+Shift+= is pressed
- **THEN** the heading `Section 30` stays at the top of the visible part

#### Scenario: Math rendered above the visible part
- **WHEN** a page is scrolled below a display math block, and the math is rendered taller than its source was shown
- **THEN** the text in view stays where it is

### Requirement: Moving list items
Ctrl+Up and Ctrl+Down in a list item (bullet, ordered or task item, at any depth, inside quotes and list items too) SHALL swap the item, together with its nested items and the blocks under it, with the previous and next item of the same list, keeping the cursor where it was in the item's text. The items of an ordered list SHALL keep the numbers in their places. Where the item has no such neighbor, and outside list items, the keys SHALL change nothing.

#### Scenario: Moving a task up
- **WHEN** the cursor is in `- [ ] b` of `- [x] a` / `- [ ] b` and Ctrl+Up is pressed
- **THEN** the list is `- [ ] b` / `- [x] a`, and the cursor is still in `b`

#### Scenario: An item with nested items
- **WHEN** the cursor is in `- a` of `- a` / `  - a1` / `- b` and Ctrl+Down is pressed
- **THEN** the list is `- b` / `- a` / `  - a1`

#### Scenario: A nested item stays under its parent
- **WHEN** the cursor is in the first of the nested items of `- a`, which is followed by `- b`, and Ctrl+Up is pressed
- **THEN** nothing changes

#### Scenario: An ordered list keeps its numbers
- **WHEN** the cursor is in `2. b` of `1. a` / `2. b` and Ctrl+Up is pressed
- **THEN** the list is `1. b` / `2. a`

### Requirement: Move item commands
The Paragraph menu SHALL offer Move Item Up (Ctrl+Up) and Move Item Down (Ctrl+Down). In the source mode they SHALL move the list item at the cursor alike.

#### Scenario: Moving in the source mode
- **WHEN** the source mode is on, the cursor is in `- b` of `- a` / `- b` and Ctrl+Up is pressed
- **THEN** the page is `- b` / `- a`
