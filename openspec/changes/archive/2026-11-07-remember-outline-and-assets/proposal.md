## Why

The outline and the assets pane are hidden in every editor window that opens, so a user who works with them has to show them again (Ctrl+O, Ctrl+D) for every page, and again after every start of the application.

## What Changes

- Showing or hiding the outline in an editor window is remembered: the next editor window that opens shows the outline if it was last shown, and hides it if it was last hidden.
- The same for the assets pane.
- Both choices are kept across starts of the application, like the editor's zoom and theme.
- Windows already open keep showing or hiding their own outline and pane; the choice applies only to windows opened afterwards.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: the outline requirement says a new window opens with the outline shown or hidden as it was last.
- `page-assets`: the pane is no longer always hidden when a window opens; it opens as it was last shown or hidden.

## Impact

- The editor's shared settings (two more of them, saved in `settings.txt`), opening an editor window, the Outline and Assets commands, editor tests, README.
