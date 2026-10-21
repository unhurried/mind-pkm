# Design

## Context

The zoom level and the scroll position live in the application, the selection in the open map's state. Opening a map always centered the root at the current zoom. The last opened map is already remembered in the per-user settings.

## Goals / Non-Goals

**Goals:**
- Restarting shows the map as it was: zoom, scroll position, selection.
- The same for any map opened again, also from the command line.

**Non-Goals:**
- Remembering the window size, folds (they are in the map file already), the filter or the search panel.
- Writing anything into the `.mm` file.

## Decisions

- **Per map, in the settings directory**: `views.txt` holds one line per map, `zoom TAB x TAB y TAB ids TAB path`, the most recently shown first, at most 50. The path comes last so that it may hold anything but a tab; maps are matched like files are compared elsewhere (case-insensitively on Windows). Storing it in the map would make Freeplane-visible changes and a dirty map out of merely looking at it.
- **Selection by Freeplane ID**: node numbers are not stable across loads, the `ID` attributes are. The focused node is written last, matching how a selection is restored (the last node gets the focus).
- **When**: on exit (every exit path goes through one helper), before another map replaces the open one, and after a successful save (so that a crash still leaves a recent view). Untitled maps have nothing to be remembered by.
- **Restoring**: the zoom is set first (rounded to a zoom step), then the selection, then the map is laid out (which drops gone nodes and moves hidden ones to a shown ancestor), then it is scrolled to the remembered position; the scrollable clamps a position beyond the map.
- **Tests never touch the real settings**: the settings directory is absent in tests.

## Risks / Trade-offs

- A scroll position from a larger window may show less of the same area in a smaller one; the focused node is not forced into view, matching what the user last saw.
