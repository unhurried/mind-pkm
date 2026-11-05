# Design

## Range

- Siblings (same parent): the children of the parent from one to the other. Their descendants are left out, as Freeplane does: a run of siblings is what is moved, deleted or formatted together.
- Anything else: the nodes the layout shows (folds and the filter applied), in document order (preorder), from one to the other. A root anchor therefore selects every shown node down to the clicked one.

The selection keeps the clicked node last, so that it is the focused node (`Doc::select` focuses the last node).

## Anchor

The anchor is remembered together with the selection the range produced. A later Shift+click uses that anchor only while the selection is still exactly that range; any other change of the selection (a click, Ctrl+click, arrow keys, Undo, a deleted node) makes the focused node the anchor again. This needs no reset at every place the selection changes. An anchor that is gone or not shown falls back to the focused node too.

## Not changed

Shift+arrow keys keep adding the target node (`Doc::extend_selection`); Ctrl+click keeps toggling, and takes precedence over Shift.
