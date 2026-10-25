# Design

## Context

The active region of the page editor is an iced `text_editor` holding the block's Markdown source; edits are spliced back into the page text, which is re-parsed after every change (`Page`). Inactive blocks are rendered from `content::inlines` (styled runs with a `SourceMap` back to the source) as `rich_text` lines, with images, math and embeds laid out by `row().wrap()` word by word.

iced 0.14 facts this design relies on (checked in the sources): `iced_graphics::text::Paragraph` exposes its `cosmic_text::Buffer`, whose layout runs give each glyph's byte range and x position and the line's baseline (`line_y`); `Renderer::fill_paragraph` takes a clip rectangle; `draw_image` and `draw_svg` draw into any rectangle; a widget can request the IME with a caret rectangle and an optional preedit (`Shell::request_input_method`), and receives `input_method::Event`s - the stock `text_editor` lets the runtime show the preedit (over-the-spot) and inserts the commit, which this widget does too; focus goes through `operation::Focusable`, so `operation::focus(id)` keeps working.

## Goals / Non-Goals

**Goals:**
- The block being edited looks like its rendering; only the inline elements at the cursor show Markdown.
- The page text stays the single source of truth; the shown text is derived from it and every edit becomes a source edit.
- The editing model is UI-free and testable; the widget only lays out, draws and turns input into messages.

**Non-Goals:**
- Character selection across blocks (unchanged: whole blocks), editing code-like blocks rendered (they keep `text_editor`), right-to-left text, on-the-spot IME preedit drawing.

## Decisions

### Shown text and units

For the active block (a page range, or a table cell's text parsed as a one-cell table), `live::Display` is built from the same `content::inlines` the renderer uses plus the block's inline elements:

- **Elements**: every inline construct with its source range - emphasis, strong, strikethrough, sub/superscript, links (also autolinks), images, code spans, inline math, footnote references (from the parser's events), pairs of supported inline HTML tags, `==`/`~`/`^` pairs (from the existing passes), and pieces whose shown text differs from their source (escapes, entities, emoji shortcodes, `<br>`).
- **Hidden source**: block bytes not shown by any run (markers, link destinations, block markers, continuation indentation). A hidden byte belongs to the innermost element containing it; bytes in no element (block markers) are always hidden.
- **Revealed elements**: those whose range contains the cursor, ends included. Their hidden bytes are shown as dimmed text, atoms (image, math, footnote number, emoji, entity) as their source text; an image or formula keeps a rendered preview right after its source.
- The shown text is a sequence of **units**: runs of characters copied one to one from the source (normal text, revealed markers), atomic units (an emoji, an entity, a non-revealed image or formula shown as one object, a line break written as `<br>`), each with its source range, style and, for objects, what to draw.

Cursor mapping: shown offset 0 maps to the first unit's source start (the block's content start when empty); any other offset maps to the source right after the unit character before it (left-attached), so a cursor at the edge of an element maps inside the element's range and reveals it. Source to shown offset: the position inside a one-to-one unit, else the shown offset where the next unit starts.

### Edits

Edits are made on shown text and turned into source edits:
- Deleting a shown range deletes the source of its units. Hidden bytes between them are deleted when their element lies entirely in the deleted range or when all of the element's shown content is deleted (the whole element goes); otherwise they are kept, so `a **bo|ld**` minus `a bo` leaves `**ld**`.
- Inserting puts the text at the cursor's source position (left-attached); since elements at the cursor are revealed, the user sees where it goes.
- Enter, Shift+Enter, Tab, Backspace at the start and Delete at the end keep the existing source-level rules (`edit::enter`, `edit::backspace`, ...), called with the cursor's source position; Backspace at shown offset 0 uses the block's content start (after the hidden block marker), so it removes the marker or joins blocks as before.
- After every edit the page is re-parsed, the region and display rebuilt, and the cursor placed from its source position. Undo snapshots stay source-level.

### The widget

`LiveText` (custom `Widget`), given the units' styled pieces, the cursor/selection in shown offsets, the find marks, and a key-binding function:
- **Measuring**: each style run is one single-line `Paragraph` (font, size, color by style); glyph clusters give byte offsets and x advances, the run's `line_y` its baseline. Objects have a size (images scaled to the column, SVG math at its typeset size). Paragraphs are cached in the widget state by run text and style, so typing re-measures only the run that changed.
- **Line breaking** (pure, unit-tested): greedy over clusters and objects; break opportunities after spaces and between CJK characters (not before `、。，．）」』】！？` and not after `（「『【`); a cluster run longer than the width breaks anywhere; trailing spaces hang. Lines align runs and objects on a common baseline (images on the baseline, math centered on the text).
- **Drawing**: per line, backgrounds (selection, find marks, `==highlight==`, code, kbd), then each run's fragment (`fill_paragraph` clipped to the fragment, shifted so the fragment starts at its x), underline and strikethrough lines, images and SVGs, then the caret (blinking like `text_editor`).
- **Input**: clicks hit-test to a shown offset (double click selects a word, triple the block's text, Shift+click extends), drags extend the selection, Ctrl+click on a link publishes the link message. Keys go through the same binding function as `text_editor` (`keys::binding`); motions that need the layout (Up, Down, Home, End, Page Up/Down) are resolved by the widget into a target offset, or into the editor's block-crossing binding when there is no line above/below; everything else becomes a message for `Page`. IME: the caret rectangle and preedit are requested on redraw; a commit inserts its text.
- **Focus and scrolling**: the state implements `Focusable`; `operate` also reports the caret rectangle through a custom operation, which the existing reveal-on-move scrolling uses instead of measuring source text.

### Integration

`Active` holds either a `text_editor::Content` (code-like blocks) or a `LiveRegion` (display + shown selection). `Page::perform` translates the `text_editor` actions the editor code already uses (insert, paste, enter, backspace, delete, moves, selections) to live edits, so the key handling, rules, auto-pairing, clipboard, find and table code keep their structure. A table cell's live region works on the cell's Markdown (`<br>` and `\|` as written) and writes back through the table model.

## Risks / Trade-offs

- [IME composition shown by the system overlay, not inside the text] → same as the current `text_editor`; on-the-spot drawing can follow later.
- [Glyph positions from single-line paragraphs ignore kerning across break points] → negligible; text is shaped per style run.
- [A large custom widget can regress basic editing] → the model and the line breaking are unit-tested; the editor's app tests drive typing, deleting, moving and selecting through the same messages; the UI is checked under Xvfb.
- [Performance of re-measuring long paragraphs] → per-run paragraph cache; only the edited run is shaped again.
