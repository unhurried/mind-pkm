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
The editor SHALL show the document as a sequence of blocks, each rendered (see markdown-rendering), except the block holding the cursor, which SHALL show its Markdown source. Moving the cursor out of a block SHALL render it again. Blocks are paragraphs, headings, single list items (without their nested lists), block quotes, code blocks, math blocks, tables, HTML blocks, thematic breaks, front matter and footnote definitions.

#### Scenario: Leaving a paragraph
- **WHEN** the cursor is in the paragraph `some **bold** text` and moves to the next block
- **THEN** the paragraph shows `some bold text` with `bold` in bold and no asterisks

#### Scenario: Entering a list item
- **WHEN** a rendered task list item is clicked
- **THEN** that item alone shows its source `- [ ] item`, and the other items stay rendered

### Requirement: Styled source of the active block
The source of the block holding the cursor SHALL be styled like the rendering where text can show it: a heading's source at the heading's size, bold and italic text in bold and italic, code in the code font, links in the link color, and Markdown markers dimmed.

#### Scenario: Heading being edited
- **WHEN** the cursor is in the heading `## Plan`
- **THEN** `## Plan` is shown at the size of a level-2 heading with `##` dimmed

### Requirement: Clicking places the cursor
Clicking a rendered block SHALL put the cursor into it at the source position of the clicked text, so that the block shows its source with the cursor where the click was.

#### Scenario: Click in the middle of a word
- **WHEN** the rendered paragraph `hello world` is clicked between `wor` and `ld`
- **THEN** the paragraph shows its source with the cursor between `wor` and `ld`

### Requirement: Moving between blocks with the keyboard
Up and Down on the first and last line of a block, Left at its start and Right at its end SHALL move the cursor into the previous or next block. Up/Down SHALL keep the cursor's column where the line is long enough.

#### Scenario: Down out of a block
- **WHEN** the cursor is on the last line of a paragraph and Down is pressed
- **THEN** the next block shows its source with the cursor on its first line

### Requirement: Converting syntax as it is typed
Typing a block syntax SHALL turn the block into that kind right away: `# ` to `###### ` at the start make a heading, `- `, `* `, `+ `, `1. ` a list item, `- [ ] ` a task item, `> ` a quote. Enter after a line consisting of ` ``` ` (with an optional language) or `$$` SHALL add the closing fence and put the cursor inside; Enter after `---` as the first line of the document SHALL start front matter; Enter after a table header row (`| a | b |`) SHALL create a table (see table-editing).

#### Scenario: Code fence
- **WHEN** ` ```rust ` is typed in an empty paragraph and Enter is pressed
- **THEN** the source reads ` ```rust `, an empty line with the cursor, and ` ``` `, and the block is a Rust code block

#### Scenario: Heading as it is typed
- **WHEN** `# ` is typed at the start of an empty paragraph
- **THEN** the block is shown at the size of a level-1 heading while the title is typed

### Requirement: Enter and Shift+Enter
Enter SHALL end the paragraph and start a new one after it (in a heading, a new paragraph after the heading). Shift+Enter SHALL break the line inside the paragraph.

#### Scenario: New paragraph
- **WHEN** the cursor is in the middle of `onetwo` (after `one`) and Enter is pressed
- **THEN** the source reads `one`, a blank line, `two`, with the cursor at the start of `two`

#### Scenario: Line break in a paragraph
- **WHEN** Shift+Enter is pressed after `one` in `onetwo`
- **THEN** the paragraph's source reads `one` and `two` on two lines and it is shown with a line break

### Requirement: Enter in lists, quotes and code
In a list item, Enter SHALL start a new item of the same kind (the next number, an unchecked box for a task item); Enter in an empty item SHALL end the list, or outdent the item when it is nested. In a quote, Enter SHALL continue the quote, and Enter on an empty quoted line SHALL leave it. In code, math and front matter blocks Enter SHALL insert a line break keeping the indentation.

#### Scenario: Next task item
- **WHEN** Enter is pressed at the end of `- [x] done`
- **THEN** a new item `- [ ] ` follows with the cursor after it

#### Scenario: Ending a list
- **WHEN** Enter is pressed in an empty bullet item at the end of a list
- **THEN** the empty item becomes an empty paragraph after the list

### Requirement: Backspace and Delete across blocks
Backspace right after a block's marker (`# `, `- `, `1. `, `- [ ] `, `> `) SHALL remove the marker, making the block a paragraph. Backspace at the very start of a block SHALL join it to the end of the previous paragraph, heading or list item, and otherwise move the cursor to the end of the previous block. Delete at the very end of a block SHALL join the next block's text in the same way.

#### Scenario: Removing a heading marker
- **WHEN** the cursor is right after `## ` in `## Plan` and Backspace is pressed
- **THEN** the block becomes the paragraph `Plan`

#### Scenario: Joining paragraphs
- **WHEN** the cursor is at the start of the paragraph `two` that follows the paragraph `one` and Backspace is pressed
- **THEN** the two paragraphs become one paragraph `onetwo` with the cursor between them

### Requirement: Indenting list items
Tab and Shift+Tab in a list item SHALL indent the item under the previous item and outdent it, together with its nested items. In code and math blocks they SHALL indent and unindent the line.

#### Scenario: Nesting an item
- **WHEN** the second item of a bullet list has the cursor and Tab is pressed
- **THEN** it becomes a nested item of the first one

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

### Requirement: Files changed on disk
When an open file changes on disk, the editor SHALL reload it if it has no unsaved changes, keeping the cursor near where it was; with unsaved changes it SHALL ask whether to reload (losing them) or keep the editor's text.

#### Scenario: Changed by another application
- **WHEN** an unchanged open page is modified by another application and the editor window is activated
- **THEN** the editor shows the new text

### Requirement: PKM commands on open pages
Edit (F2) renaming a page or a directory, Add Assets and Delete (F11) SHALL first save the open editors of the affected files that have unsaved changes. Afterwards a renamed page's editor SHALL show the renamed file with its updated text, Add Assets' page SHALL be reloaded, and the editors of deleted files SHALL close.

#### Scenario: Renaming an open page
- **WHEN** page `A` is open with unsaved changes and renamed to `B` with F2
- **THEN** the changes are saved, `B.md` holds them with `A.assets` references renamed, and the editor shows `B.md`

#### Scenario: Deleting an open page
- **WHEN** page `P` is open in an editor and deleted with F11
- **THEN** its editor window closes without asking
