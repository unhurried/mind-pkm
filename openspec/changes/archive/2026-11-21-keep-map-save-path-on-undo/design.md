# Design

## Context

Each undo and redo entry holds a full map, including the path used by Save and by relative links. A successful Save As currently updates only the live map.

## Decisions

When a save to a different path succeeds, give every undo and redo entry that path too. The Next Steps refresh can use a historical map's base directory before anyone restores that entry, so its path needs to be current while it is in history. The entries retain their node contents, selection and order.

An ordinary save to the same path avoids walking the history. A failed write returns before the destination changes.
