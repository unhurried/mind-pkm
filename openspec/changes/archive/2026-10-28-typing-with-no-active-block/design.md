# Design

The key handler that runs while nothing is active placed the cursor at the page's selection (offset 0 after opening) and returned, so the key itself was never typed. The cursor stayed at offset 0, which in a heading is before `#`, a part of the source the rendered block does not show, so later inserts went before the marker.

- After placing the cursor, a cursor in hidden source is moved to where it shows (shown offset back to source, as a click there does): the start of the heading's text.
- The typed text then goes through the same edit action the active block's widget sends, so typing rules (auto-pairs, markers) apply as usual.
- Home and End start editing like the arrow keys and then move to the line's start or end.
