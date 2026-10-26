# Proposal

## Why

The user set this input policy for the page editor:

- A block element is started with its Markdown syntax at the start of a line. Once the element is determined, typing continues in its rendered form, and coming back to it later edits it rendered too (typing `* ` at the start of a line shows a list item, which stays a list item while edited, not its source).
- An inline element is started with its Markdown syntax anywhere. Once it is determined it is shown rendered; when the cursor reaches it, it shows its source, which can be edited as source, still styled like the element.

Checking the editor against it:

- **A bare marker already counts as the element.** CommonMark reads `#`, `-`, `*`, `1.`, `>` and `- [ ]` alone as empty elements, so the marker vanishes as soon as it is typed, before the space that determines the element; `#tag` or `-x` at the start of a line flicker through a heading or list item.
- **Syntax that a whole line makes switches the block to source while it is typed.** ` ``` `, `---`, a setext underline, `[toc]` or an HTML tag at the start of a line turn the block into a code block, thematic break, heading, table of contents or HTML block on the spot; an unclosed fence also turns the rest of the page into code until Enter adds the closing fence.
- **Blocks inside quotes and list items are not edited rendered.** A quote holding a list, code or a table, and a list item holding code or a table, are edited as their whole source (typing `- ` in a quote turns the quote into source); a heading or a quote inside a quote or list item loses its look while edited.
- **A paragraph with display math (`$$...$$` among text) is edited as source.**
- Inline elements already follow the policy: they are rendered once complete and show their source, dimmed markers around styled content, while the cursor touches them.

## What Changes

- **Markers determined by their space**: a block marker typed at the start of a block's text (`#`...`######`, `-`, `*`, `+`, `1.`/`1)`, `>`, `[ ]`/`[x]` in a list item) shows as typed text until the space after it is typed; then the block is that element, edited rendered with the marker hidden.
- **Line syntax determined by Enter**: a line that makes a code fence (with its info string), a thematic break (`---`, `***`, `___`), a setext underline, `[toc]`, an HTML block or an indented code block shows as typed text, without affecting the rest of the page, until Enter is pressed or the cursor leaves the block; Enter then completes it as before (closing fence, front matter, a new paragraph after the thematic break...).
- **Blocks in quotes and list items are blocks of their own**: every block inside a quote, and every block of a list item after its first one (nested lists already are), is shown and edited like a top-level block inside the quote's bar or the item's indentation: a list in a quote is edited as list items, a heading in a quote keeps its size. Typing `- ` in a quote starts a list in it. Enter, Shift+Enter, Backspace, Delete and Tab work inside the quote or item as at the top level; Enter in an empty new paragraph inside a quote or list item leaves it; Backspace at the start of the first block of a quote takes that block out of the quote.
- **Display math in a paragraph** is edited rendered: the formula is shown rendered and, at the cursor, as its source in front of its rendering.
- **Unchanged** (as decided with the user for the built-in editor): code blocks, math blocks, diagrams, front matter, HTML blocks, thematic breaks and `[toc]` show their source while the cursor is in them, like an inline element at the cursor; tables are edited cell by cell.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: the live preview covers blocks inside quotes and list items; converting syntax as it is typed waits for the space after a marker or for Enter after line syntax; Enter and Backspace inside quotes and list items.

## Impact

- `src/markdown/parse.rs`: containers (quotes, list items) around blocks; every block of a quote and the later blocks of a list item become blocks of their own; parsing with one byte masked (for a marker not determined yet).
- `src/markdown/edit.rs`, `format.rs`: the editing rules applied inside containers (their line prefixes taken off and put back), new paragraphs inside containers, leaving a container.
- `src/app/editor/page.rs`: the undetermined marker of the block being edited; `view.rs`: quote bars and item indentation around blocks.
- Tests for the segmentation, the rules inside containers and typing markers.
