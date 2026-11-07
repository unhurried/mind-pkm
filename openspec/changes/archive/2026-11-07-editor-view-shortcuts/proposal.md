## Why

The page editor's most used View commands sit on awkward keys (Ctrl+Shift+1 for the outline, Ctrl+Shift+2 for the assets pane, which needs a special case for keyboard layouts), while Open... is on the plain Ctrl+O. Ctrl+O and Ctrl+D are easier to reach for showing the outline and the assets pane.

## What Changes

- **BREAKING**: View → Outline moves from Ctrl+Shift+1 to Ctrl+O.
- **BREAKING**: View → Assets moves from Ctrl+Shift+2 to Ctrl+D.
- **BREAKING**: File → Open... moves from Ctrl+O to Ctrl+Shift+O.
- **BREAKING**: Edit → Select Word loses Ctrl+D (it stays in the menu; a double click still selects a word).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: the shortcuts of Open..., Select Word and the outline.
- `page-assets`: the shortcut of the assets pane.

## Impact

- `src/app/editor/keys.rs` (menu entries, key bindings, tests), editor tests that pressed the old keys, README's shortcut table and assets pane description.
