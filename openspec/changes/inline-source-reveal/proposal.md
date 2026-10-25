# Proposal

## Why

The page editor shows the whole block holding the cursor as Markdown source: moving into a paragraph turns every `**`, `[...](...)`, `#` and `- ` in it into visible syntax, and the block's layout jumps. The user wants Typora's behavior instead: the block being edited stays rendered, and only the inline element at the cursor shows its Markdown.

iced's `text_editor`, which the editor uses for the active block, can style text only with a color and a font per range: no underline, strikethrough, background, smaller sub/superscript, inline images or math. Hiding the markers inside it would still leave the block looking different from its rendering, and the user chose, after weighing the risks, to replace it with an editing widget of our own.

## What Changes

- **Rendered editing**: paragraphs, headings, list items, quotes, footnote definitions and table cells are edited in place in their rendered form, with a text cursor and selection drawn over the rendered text.
- **Inline source at the cursor only**: the inline elements touching the cursor (emphasis, strong, strikethrough, inline code, links, images, inline math, highlight, sub/superscript, inline HTML elements, footnote references, escapes, entities, emoji shortcodes) show their Markdown, markers dimmed; everything else in the block stays rendered. Moving the cursor away renders them again.
- **Block markers hidden**: `#`, list markers, task boxes and `>` are never shown while editing; the heading keeps its size, list items their bullet, number or checkbox, quotes their bar. Backspace at the start of such a block still removes the marker.
- **Images and math in the block being edited** are shown rendered; at the cursor, their source is shown in front of the rendered image or formula.
- **Editing keeps the markup**: deleting part of a formatted text keeps its formatting; deleting all of it removes the markers too.
- **Unchanged**: code blocks, math blocks, Mermaid diagrams, front matter, HTML blocks, thematic breaks, link reference definitions and `[toc]` still show their source while edited (with the math and diagram previews); source mode (Ctrl+/) is unchanged; selections across blocks still select whole blocks.

This reverses adjustment 1 of the `built-in-markdown-editor` change ("source is revealed per block, not per inline element").

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: the live preview, the styling of shown source, clicking, keyboard movement and Backspace at block markers describe editing in the rendered block with source shown per inline element; a new requirement covers editing rendered text.
- `table-editing`: the cell being edited is rendered with source shown per inline element.

## Impact

- New UI-free module for the display of a block being edited (`src/markdown/`): which source bytes are shown, which inline elements exist and which are revealed, mapping between shown text and source, edits on shown text turned into source edits.
- New iced widget for editing rendered text (`src/app/editor/`): line layout of styled text, images and math; caret, selection, mouse, keyboard through the existing key bindings, IME through iced's input method requests, focus, and the caret position for scrolling.
- `Page`/`Editor` use the widget for the block kinds above; `text_editor` remains for code-like blocks and source mode.
- Tests: the display model (unit and property tests), the layout's line breaking and hit testing, the editor app tests reworked from source-showing to rendered editing.
