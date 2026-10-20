# Design

## Context

Undo keeps whole copies of the map. A new node is created with a checkpoint, then its text is asked for; cancelling went back to the last checkpoint. Background results (the Next Steps refresh) are applied whenever they arrive and recorded as checkpoints too, dialogs or not. Nodes are numbered by their slot in the map's node list; a restored copy has a shorter list, so the next node created takes a slot an undone node had.

## Goals / Non-Goals

**Goals:**
- Cancelling a new node removes that node and nothing else, also from the undo history.
- A node number, once handed out, is never handed out again while the map is open.

**Non-Goals:**
- Holding back background results while a dialog is open.
- Changing what the undo history records otherwise, or its depth.

## Decisions

- **Checkpoints are identified**: every undo entry carries a serial number; taking a checkpoint returns it, and the new node's dialog remembers the serial of the node's creation.
- **Cancelling**: when the creation is still the latest entry, the map goes back to it as before (which also restores the parent's fold). Otherwise the creation entry is dropped and the node is deleted from the current map and from every entry recorded after the creation, so undoing the later changes never brings the empty node back; the later changes themselves stay.
- **Numbers are never reused**: whenever an older copy replaces the current map (undo, redo, cancelling), its node list is padded with empty slots to the length of the list it replaces. Since every node is created in the current map, its length is the high-water mark of the numbers handed out, and the padding carries that mark over. Alternatives - a global counter, or IDs independent of slots - would change every lookup for no extra safety.
- **Tests at the document level** (no window needed): cancelling with and without a recorded change in between, undo afterwards, and node numbers after undo and redo.

## Risks / Trade-offs

- Padding costs one empty slot per undone node; negligible next to the map copies the history keeps.
- Removing the node from later undo entries relies on node numbers not being reused, which this change guarantees.
