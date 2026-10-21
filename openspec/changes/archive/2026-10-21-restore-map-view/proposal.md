# Proposal

## Why

After a restart the last map is opened again, but at 100% with the root selected and centered, so the user has to find their place again every time.

## What Changes

- The application remembers, per map file, the zoom level, the scroll position and the selected nodes as they were when the map was last shown (when the application exits, another map takes its place, or the map is saved).
- Opening a map with a remembered view restores it instead of centering the root. Selected nodes that no longer exist are left out (the root is selected if none is left); a selected node hidden by the filter or a fold hands its selection to its nearest shown ancestor.
- The views are kept in the per-user settings directory, never in the map file.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `application-shell`: A new requirement for restoring the view; opening maps refers to it; the per-user settings keep each map's view.

## Impact

- `src/app/settings.rs`: a `views.txt` settings file.
- `src/app/file.rs`: remembering the view on exit, on switching maps and on saving; restoring it on opening.
