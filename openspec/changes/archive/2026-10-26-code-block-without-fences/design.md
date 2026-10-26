# Design

## Context

The active block is shown either rendered (`live::Display`, `LiveText`) or, for code-like blocks, as its source in an iced `text_editor` whose content is the block's region; each edit of the content is spliced back over the region. Rules inside quotes and list items (`edit::nested`) already take container prefixes off a run of lines and put them back on the lines a rule changes (`Inner`).

## Decisions

- **`CodeBody`** (`edit.rs`): the lines between the fences (all lines of an indented block) as an `Inner` whose prefixes are the containers' plus the fence's indentation (modelled as an item container of that width, which consumes up to that many spaces, as CommonMark does), with exact blank lines (a line of spaces is code, not blank). It maps page offsets to code offsets (an offset on a fence goes to the code's start or end) and back, and writes new code back: lines left as they were keep their prefixes, others get the canonical ones. A fenced block without lines (` ``` ` / ` ``` `, or an unclosed fence at the end) gets new lines inserted before its closing fence (after its opening one). It also holds the info string and its range.
- **The active code block's `text_editor` holds the code**: `Page::place` builds the body and maps the selection; `perform` sends edits through `CodeBody::replace` and reparses; if the block is no longer the same code (a closing fence typed inside it), the region is placed anew. The rules (Enter, Tab, auto-pairing) keep working on the page text with page selections, and the body is rebuilt after them.
- **Highlighting**: the Markdown highlighter gets a setting for plain code of a language, starting in the fenced-code state with a fence that never closes.
- **View**: the code box (code font at the rendered size, padding, background) around the text editor, and a small text input for the language at its bottom right; entering a language replaces the info string (one undoable change per edit).
- **Keys**: Left/Right at the code's start/end cross into the neighbors as before (the edge is the code's, not the region's); Backspace at the code's start uses the block-start rule (to the end of the previous block), and in empty code replaces the block by a new paragraph; Down (and Right) at the end of the last block of the page starts a new paragraph after it, for any block shown in a text editor.

## Risks / Trade-offs

- [Typing a fence line inside the code ends the block there] → that is what the text then says; the region is placed anew from the parse.
- [Tabs before an indented block's code are kept as code] → only spaces count as its indentation in the body; a tab-indented line shows its tab.
