# Proposal

## Why

Exploratory testing turned up four places where the cursor or the selection ends up somewhere the user did not put it:

- Cancelling a new sibling node (Enter, then Esc or an empty text) selected the parent instead of the node the sibling was added after, against the mindmap-editing spec ("the previously selected node is selected again"). With the root as the parent, a following Ctrl+E renamed the root.
- Ctrl+T (Table → Insert Table) left the cursor after the inserted table, in its last body cell, so what was typed next went into the bottom right cell.
- After a quick click into a rendered block (the press and the release arriving together, as with a short click), the page still took the button for held. Moving the pointer then selected text from the cursor to the pointer (seen when a link was Ctrl+clicked), and a text editor shown later in that block's place - a paragraph `$$` / `\frac{1}{0` / `$$` completed into a math block by typing `}` - was handed the old click, so the cursor jumped back to where the block had been clicked and the next characters went there.

## What Changes

- Cancelling a new node selects again the nodes selected before it was created (those still there, else its parent), also when a refresh changed the map meanwhile.
- Insert Table puts the cursor in the first header cell.
- The page notices the release of the button after any press on it, also when the release comes with the press, so a click always ends with its release.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: the selection after cancelling a new node (a new requirement, so that it does not overlap the open change `edit-nodes-in-place`, which modifies Structural editing).
- `table-editing`: the cursor after Insert Table (a new requirement).
- `page-editor`: a quick click ends with its release (a new requirement).

## Impact

- `src/app/doc.rs` (`cancel_new_node`), `src/app/node_edit.rs`, `src/app/editor/mod.rs` (`insert_block`), `src/app/editor/press_relay.rs`.
- Tests in `src/app/tests.rs`, `src/app/editor/tests/tables.rs`, `src/app/editor/tests/editing.rs` and `src/app/perf.rs` (the headless interface, for a press and a release in one batch of events).
