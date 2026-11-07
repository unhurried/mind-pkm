## Context

The editor windows share `EditorPrefs` (built-in editor, theme, zoom, auto-pairing, window size), loaded from `settings.txt` at start and saved line by line when one of them changes (`Effect::SetPrefs`). Each window keeps whether its outline is shown (`outline`) and its assets pane (`assets: Option<AssetsPane>`); both start hidden.

## Decisions

- Two more shared settings, `outline` and `assets`, saved as `editor_outline=0|1` and `editor_assets=0|1` in `settings.txt`. Missing (as for every existing user), both are off, so nothing changes until the user shows one of them.
- Toggling the outline or the pane in a window sets the shared setting to the window's new state (`Effect::SetPrefs`), so the last toggle in any window wins. Other open windows are not changed: the settings are read only when a window opens.
- A window opens with the outline shown and the pane created (empty, to be read) as the settings say. Opening the window starts the background read of the pane's folder itself: a page opened at the start (from the command line) gets no message before it is shown, and the reads are otherwise started only after a message.
- Closing a window does not change the settings: they follow the user's last explicit choice, not the state of the last window closed.
