# Proposal

## Why

The PKM commands can only be reached by function keys (the add-on's assignments). Ctrl shortcuts people know from other applications - Ctrl+F for search - are faster to find and type.

## What Changes

- Ctrl aliases for PKM commands, the function keys staying as they are: Ctrl+O Open (F1), Ctrl+Shift+F Filter (F5), Ctrl+F Search (F6), Ctrl+N New Page (F9), Ctrl+Shift+N New Directory (F10).
- **BREAKING**: File → New and File → Open lose their Ctrl+N / Ctrl+O shortcuts (the menu entries stay).
- While typing in the search keyword box, Ctrl+F reaches the map too (it focuses the box again).

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `application-shell`: shortcuts of the File and PKM menus.
- `document-search`: Ctrl+F shows the search panel.
- `map-filter`: Ctrl+Shift+F asks for a filter command.

## Impact

- `src/app/keys.rs`: key bindings; Ctrl+F passes the search box.
- `src/app/view.rs`: menu shortcut labels.
- `README.md`: shortcut tables.
