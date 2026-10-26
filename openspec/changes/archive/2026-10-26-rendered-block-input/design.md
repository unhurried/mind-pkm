# Design

## Context

The page is parsed with pulldown-cmark after every change (`Document::parse`) and segmented into blocks: top-level elements, with lists flattened so that every list item is a block (`List::flatten`, nested lists by depth, content after a nested list as a continuation). A quote is one block whatever it holds. The block holding the cursor is edited rendered (`live::Display`, `LiveText`) when `live::editable` allows it - paragraphs, headings, list items, quotes and footnote definitions holding no code, list, table, HTML, rule or display math - and as its source in a `text_editor` otherwise. The editing rules (`edit::enter`, `shift_enter`, `backspace`, `delete`, `indent`, `format::set_heading`, `toggle_list`) are pure functions over the page text, the parse, the active region and the selection, and know about `#`, list markers and `>` at the start of a line (`line_marker`).

## Goals / Non-Goals

**Goals:**
- A marker shows as typed until the element is determined (the space after it, or Enter for line syntax); then the block is edited rendered.
- Blocks inside quotes and list items are edited rendered like top-level ones, with the same rules.
- The text stays the truth: what is shown while a marker is undetermined never changes the text.

**Non-Goals:**
- Editing code-like blocks rendered (unchanged: source at the cursor).
- Tables inside quotes or list items cell by cell (they stay source while edited, as now).
- Footnote definitions holding lists or code (they stay one block, as now).

## Decisions

### Containers

`Document` gets `containers`: the quotes and the list items whose content holds blocks of their own, each with its kind (quote; item with its depth and content column) and range. Each block gets `outer`, the indices of the containers around it, outermost first. Segmentation walks the events recursively:
- a quote is a container: each of its children is segmented like a top-level element (a list flattened, a nested quote a container, anything else a block) with the quote added to `outer`; an empty quote (`>` alone) is a `Quote` block;
- a list item keeps its first child (and the paragraphs that follow it) in its `ListItem` block, nested lists are flattened as before, and any other child after the first (a code block, quote, table, heading, thematic break, HTML, math) is segmented with the item as a container; paragraphs after such a child are a continuation, as after a nested list.

A block inside a container starts at the start of its first line (so the line's `>`/indentation belongs to it, as an item's indentation does now); the live display already hides source no element owns (block syntax). Lines holding only container syntax (`>`) between blocks count as blank. Headings inside quotes appear in the outline.

Alternative considered: keeping a quote as one block and drawing its inner list items and headings inside the live widget. The widget has no per-line indentation or markers, and every rule would need to know the structure inside the block; flattening reuses the block machinery for everything.

### Rules inside containers

`edit::nested(text, doc, region, sel, rule)` applies a rule to a block with containers: it takes the lines of the innermost container (for an item, the lines after its first one), removes the containers' prefixes from each line (`>` and one space for a quote, up to the content column of spaces for an item; a lazy line keeps what it has), parses that text, applies the rule as at the top level, and puts the result back: unchanged lines keep their original prefix, changed and new lines get the canonical one (`> ` or the item's indentation, `>` alone on blank quote lines). The selection and a new paragraph (`Pending`) are mapped back; a new paragraph's line breaks get the prefixes too, so the paragraph Enter adds after a quoted paragraph is `\n>\n> ...`. `Pending` remembers its containers so that the view draws it inside them.

The rules get two container-specific cases: Enter in an empty new paragraph inside a container moves it out of the innermost container; Backspace at the start of the first block of a quote (no block before it inside) removes the quote's prefix from the block's lines (and turns a following `>`-only line into a blank one), while at the start of the first block of a list item after its text it joins that text, the block before it.

A list item's later blocks include its later paragraphs, which were part of the item's block (or a "continuation" item block after a nested list): every one of them is now a block inside the item, so the rules above apply to them uniformly, and the item's block is its first block only.

A line break typed at the very end of a block (Shift+Enter) stays in the block: the region holding the cursor is extended over it after an edit, and the display shows it as a line to type on (before, the cursor fell into a new paragraph).

`format::set_heading`, `step_heading` and `toggle_list` go through `nested` when the blocks they change share their containers; `toggle_quote` stays line-based (it unquotes quoted lines).

### Undetermined markers

`Page` keeps `tentative`: the offset of one byte of the active block that the parse doesn't see (it is replaced by `x` for parsing; text events covering it get their real text back, so the block shows what was typed). After an edit of a live block, with the new text parsed as it is:
- if the block at the cursor keeps its kind and containers, nothing is masked;
- if the edit typed whitespace after a complete marker (`line_marker` covering the whole line after the containers' prefixes), the element is determined: nothing is masked;
- if the block at the cursor is live and shows text from the cursor's line, it is determined by its content (`>x` is a quote): nothing is masked;
- otherwise the first byte of the typed segment (the block's content start on its first line, the start of the cursor's line after the containers' prefixes on another) is masked and the page parsed again with it.

The mask is dropped (and the page parsed as it is) when the cursor leaves the block, when an editing rule (Enter...) or undo changes the text, and when the block is deactivated. While masked, everything else - rendering, outline, Enter rules - sees the masked parse, so ` ``` ` is a paragraph until Enter, where the fence rule (which reads the text) adds the closing fence.

### Display math

`live::editable` stops excluding display math: the display already shows it as an object (centered, its source in front of the rendering at the cursor), and the widget lays it out.

## Risks / Trade-offs

- [Rules inside containers rewrite the prefixes of the lines they change to the canonical form] → only changed lines; lazy lines elsewhere keep theirs.
- [A marker left undetermined when the cursor leaves becomes what CommonMark says (`#` an empty heading)] → that is the text; the display just stops pretending.
- [Segmentation changes affect rendering, navigation, find and counts] → existing tests keep running; new tests cover quotes with lists, headings, code in items, and ranges/gaps.
