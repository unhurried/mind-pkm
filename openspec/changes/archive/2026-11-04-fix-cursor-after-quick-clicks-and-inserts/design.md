# Design

## Cancelling a new node

`Doc::cancel_new_node` already restored the map, the selection and the unsaved mark when nothing had happened since the node was created; the node edit then selected the node's parent anyway, which is the previous selection only for a new child. The selection now always comes from the checkpoint taken before the node: when a refresh landed meanwhile and the node is deleted from the current map instead, the checkpoint's selection is taken again, leaving out nodes the refresh removed, and the parent only when none is left. The node edit no longer selects anything itself.

## Insert Table

`insert_block(open, close)` puts the cursor between `open` and `close`. A new `insert_block_with_cursor(block, cursor)` takes the cursor as an offset into the block (mapped through the prefixes of the containers, as before); Insert Table uses it with the offset of the first header cell, between the spaces of `|  |`, where the table editor finds cell (0, 0). The other blocks keep `insert_block`.

## Quick clicks

The editor knows a press is held from the press itself (`block_drag`, `press_at`, set when a rendered block or its text is pressed) until `EditorMsg::Released`, which came only from a subscription that listens for the release while something is held. iced hands a window the events that arrived since the last update in one batch: when the press and the release come in the same batch, the subscription is only started after both were handled, and the release is never seen. `block_drag` stayed set, so:

- the live text widget, told the button is held, took every later pointer move over it for a drag and selected text up to the pointer;
- `PressRelay::relaying`, wrapped around a text editor that replaces the block, replayed the held press as a click on that editor as soon as one was shown - in the case found, after typing turned the paragraph into a math block edited as its source.

`PressRelay::tracking`, which already wraps the page's column and tells where a left press lands, now also remembers that the left button went down and publishes `EditorMsg::Released` when it sees the release. It sees every mouse event of the window (iced passes them to all widgets), in order, in the same batch as the press. The subscription stays for releases the tracking widget does not get (outside the window, or in source mode, where the page column is not shown). A second `Released` changes nothing: it clears what the first cleared, and `assets_released` takes the dragged item, so a drop happens once.

The test drives the editor window's headless interface with a press and a release in one `update` call, as iced does, and checks that the editor no longer takes the button for held.
