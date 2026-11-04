# Proposal

## Why

The page editor writes a list's items sometimes with blank lines between them and sometimes without, often in the same list. Each editing rule decides on its own: Enter separates a new item the way the list looks at that moment, Backspace into the previous item and every inserted block always add blank lines, and nothing takes them away again. In Markdown a list is either tight or loose as a whole (one blank line makes every item spaced), so the source ends up mixed, and a list often turns loose although nothing in it needs blank lines. A code block put under an item is the common case: `* a` / `` ``` `` ... is valid without blank lines, but the editor always adds them, so the whole list spreads out. Inserting a table, code block, math block or rule while a list item is active even splits the list in two, because the block goes after the list item outside the list.

## What Changes

- A list the editor edits is written in one of two forms, the same for all of its items: *tight* (no blank lines between its items, nor before the blocks inside its items) or *loose* (exactly one blank line in each of these places). Nested lists decide for themselves.
- A list is written loose only when it has to be: when a block inside one of its items would not stay that block without a blank line before it (a paragraph, an image or math on a line of its own, a `---` rule). Otherwise it is written tight - code blocks, tables, quotes, headings and nested lists inside items keep a list tight. When the block that needed the blank lines goes, the list becomes tight again. A list written loose elsewhere that needs no blank lines becomes tight when it is edited.
- The form is applied to the list that an edit changed, after an editing rule (Enter, Backspace, Delete, Tab, inserting a block, pasting) and when the cursor leaves a block whose text was typed in. Lists the user does not edit are left as they are; opening or saving a page changes nothing.
- Inserting a code block, math block, horizontal rule or table (and pasting cells as a table) while a list item is active puts the block into that item, below its text, instead of after the list item outside the list.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: new requirement for the blank lines of edited lists; blocks inserted while a list item is active go into the item; the scenario of a paragraph made from an item states the blank lines.
- `table-editing`: Creating a table - Insert Table while a list item is active puts the table into the item.

## Impact

- `src/markdown/parse.rs` (the lists of a page and where their items and blocks are separated), `src/markdown/edit.rs` (writing a list tight or loose, a new block inside a list item), `src/app/editor/page.rs` (applying it after edits and when leaving a block), `src/app/editor/mod.rs` (inserting blocks), with tests in `edit.rs`, `parse.rs` and `src/app/editor/tests/`.
- No change to the files' format otherwise: only blank lines between the blocks of an edited list change, and the page is never rewritten as a whole.
