# Design

## Context

A list item holding blocks after its text is a container: its later blocks are blocks of their own, indented to the item's content column after a blank line. A new empty paragraph has no source until typed into (`Pending`); typed block syntax (table header row and Enter, code fences, markers) already works in such a paragraph inside quotes and list items.

## Decisions

- **The item the block joins.** Going back from the item, the nearest item at its depth or above decides: an item of the same list at the same depth (the previous sibling), or the item one level up whose nested items include it (its parent). Anything else (another list, the first item at the top) keeps removing the marker. An empty item takes no blocks (it can't start with a blank line), so it is never joined.
- **An empty item becomes a new empty paragraph.** Its line goes, with the blank lines before it, and the cursor is in a `Pending` paragraph after the item it joins, indented to that item's content column after a blank line. Nothing is written until something is typed, so Backspace again simply takes it back to the end of the item, and Enter leaves the item as for any empty paragraph in it.
- **An item with text keeps its text.** Its marker is replaced by the indentation of the content column of the item it joins, after a blank line (unless there is one already), so it is a paragraph of that item; its own nested lines stay as they are.
- **Task items** still lose their box first; the next Backspace applies this rule.
- Inside quotes and list item blocks the rule runs on the container's lines without their prefixes, like the other rules, so `>` and indentation are kept.

- **Tables keep their containers' prefixes.** The table being edited is parsed from its lines without the prefixes of the containers it is in, and written back with them (a quote's `> `, an item's indentation) on every line. A table in a quote is recognized by the page's parse, since its source alone reads as a quote. The cell at an offset skips those prefixes before the leading pipe.

## Risks / Trade-offs

- A list whose item gets a second block becomes loose (a blank line inside an item), which is what Markdown requires for a paragraph after the item's text.
- Making a middle item a paragraph of the previous one, rather than taking it out of the list, differs from the earlier marker removal; that removal left a continuation line merged into the previous item's text, which was never what was meant.
