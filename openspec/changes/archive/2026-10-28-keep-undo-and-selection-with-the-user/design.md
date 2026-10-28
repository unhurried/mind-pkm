# Design

## The refresh and the undo history

The refresh's changes are applied to the current map and to every map the undo and redo steps hold, instead of being recorded as a step. Undoing then goes back to the map before the user's last change as the refresh would have made it, so neither the refresh nor the user's change is lost. The refresh skips nodes that don't exist or have another text in a map, so applying it to older maps is safe. Nodes it creates in one map get ids above every id handed out in any of them, keeping ids unique across the history.

## Selections in the history

Each undo step keeps the selection of the map it holds (the selection at the time of the change, before it), and each redo step the selection when it was undone; Undo and Redo restore it with the map. The unsaved and edited marks are kept too, so that cancelling a new node right away restores them.

## A selected node that is gone

The map as last laid out is kept. When the focused node no longer exists, the node nearest to where it was there is selected: a sibling that still exists (the next one first), else the parent; if the parent went too, the same for the parent.

## Syncing children

The children of a page node are reconciled with its Next Steps items: for each item in order, the first remaining child with that text is moved into place (or a node is added), and the children left over are deleted. The same applies to Due Today and Past Due.
