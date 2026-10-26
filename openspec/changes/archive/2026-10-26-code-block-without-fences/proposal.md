# Proposal

## Why

Following the input policy (a block element, once determined, is edited in its rendered form), the user asked for code blocks to be edited without their fences too: the code alone, as it is rendered. Math blocks, diagrams, front matter, HTML blocks, thematic breaks and `[toc]` keep showing their source at the cursor.

## What Changes

- **Code blocks are edited as their code**: the block with the cursor stays the code box (code font, syntax highlighting for its language) and shows only the code; the fences, the indentation of an indented block or fence, and the prefixes of the quote or list item it is in are hidden, and new lines get them.
- **Language field**: the box shows the language (the info string) in a field; changing it rewrites the opening fence.
- **Leaving and removing**: Backspace at the start of the code moves to the previous block; Backspace in an empty code block turns it into an empty paragraph; Down at the last line of a code block that ends the page starts a new paragraph after it (with the fences hidden there is no other way below it from the keyboard).
- Mermaid diagrams (` ```mermaid `) are unchanged: their source, fences included, with the diagram below.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: the live preview shows a code block's code without its fences; a new requirement covers editing code blocks.

## Impact

- `src/markdown/edit.rs`: the code of a code block as shown, mapped to and from the page (the prefix machinery the rules inside containers use), and edits written back keeping the prefixes of untouched lines.
- `src/app/editor/page.rs`: the active code block's text editor holds its code; `view.rs`: the code box with the language field; `highlight.rs`: highlighting plain code of a language; `mod.rs`: Backspace in an empty block, Down after the last block, the language field.
- Tests for the mapping (fenced, indented, empty, unclosed, in a quote or list item) and the editor flows.
