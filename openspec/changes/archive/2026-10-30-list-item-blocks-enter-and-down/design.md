# Design

## Context

A list item's first block is its text; Enter at its end inserts a new item marker. The blocks the item has below its text used to be kept below the new marker, so they became the new item's. That works for nested lists (an empty item followed by a nested list parses as such), but any other block becomes the empty item's first block: its block then covers that code block or paragraph, and typing into the item joins a paragraph below as a lazy continuation.

## Decisions

- **The first block below the text decides.** If it is a nested list item, the blank lines before it go and it moves under the new item (unchanged). Otherwise the new item is inserted after the whole item (its end with all its nested content), so the blocks stay where they are. Text after the cursor still moves into the new item together with what follows it.
- **Down below a container's last code-like block stays in the container.** When Down cannot move within a code-like block (shown as code or source) and the next block is not inside the innermost quote or list item the block is in, a new empty paragraph is started in that container, as Down at the end of the page already does. The empty paragraph has no source until typed into, so Down again simply moves on to the next block and leaves the page unchanged.

## Risks / Trade-offs

- A new paragraph between an item's text and the code block below it can no longer be started with Enter at the end of the text; it is reached by Enter at the end of an earlier paragraph of the item, or by typing above the code.
