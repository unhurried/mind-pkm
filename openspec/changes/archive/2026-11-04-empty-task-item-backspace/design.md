# Design

## Decision

`backspace` decides on the marker before the cursor. For a task item it removed only the box, and only a plain item went into the item before (`into_previous_item`). Now an item whose text after its marker (the box included) is empty is handled as a plain item: it goes into the item before it, or, when there is none, its whole marker (bullet and box) is removed. `into_previous_item` already treats the box as part of the marker for an empty item, so nothing else changes.

Only empty items are affected: removing just the box of a task item with text stays the way to turn a task into a plain item.
