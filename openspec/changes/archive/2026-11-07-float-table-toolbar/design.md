# Design

## Context

The table being edited is built by the page editor's view of the active region as a column of the toolbar and the grid of cells, so the active region is taller than the rendered table by the toolbar's height plus the spacing. The page column keeps the entry at the top of the visible part in place across layouts, which does not help here: the table is that entry or below it, so its cells move down either way.

The editor window's menus, right-click menus and dialogs are layers of a stack at the window's root, not iced overlays. Anything drawn as an iced overlay is drawn above all of them and takes the pointer events first.

## Goals / Non-Goals

**Goals:**
- The active region of a table is exactly as tall as the grid: entering or leaving a table changes no block's place.
- The toolbar stays usable (its buttons and its two text fields take presses and keys) and stays within the page's visible part.

**Non-Goals:**
- Changing the toolbar's commands or the other ways to reach them (Table menu, right-click menu).
- Any change to the rendered (not edited) table, or to the cells' own heights.

## Decisions

### A wrapper widget that lays the toolbar out beside the table and shows it as an overlay

A small widget (`floating.rs` under `src/app/`, next to the other wrapper widgets) takes the content (the grid) and the bar (the toolbar). Its layout is the content's; the bar is laid out too (with the content's width as its maximum, so a narrow table does not crush it) but takes no room. Its base `update` / `draw` / `mouse_interaction` are the content's alone; `overlay` returns the content's own overlay when there is one, else an overlay holding the bar at the place worked out from the content's bounds and the viewport. `operate` reaches both, so the bar's text fields can be focused like any other.

The bar's widget tree stays a child of the wrapper whether or not the bar is shown, so a resize field being typed in keeps its text and focus when the bar is hidden for a moment (a menu opened) and shown again.

Alternatives considered:
- iced's `float` with `translate`, stacked over the grid: its translation is given only the bar's own bounds and the viewport, not the table's, so it cannot stop the bar at the table's bottom when the table scrolls out of view - the bar would stay stuck at the top of the page over other blocks. And a stacked layer is limited to the grid's width.
- A `pin` at a negative offset inside a stack: it is drawn over the block above, but that block's mouse areas get the presses before it (a column hands events to its children in order), so clicks on the toolbar would also reach the paragraph under it.

### Where the bar goes

A pure function of the table's bounds, the bar's size and the viewport (the page's visible part, as the page's scrollable hands its children) gives the bar's top-left, or nothing:

- normally the bar's bottom is a small gap above the table's top, its left the table's left;
- it is pushed down to the top of the viewport when that is lower (sticky), but no lower than the table's bottom minus the bar's height;
- it is not shown when it would not lie wholly within the viewport. This also keeps an overlay from taking presses meant for the menu bar or the find bar above the page, since an overlay takes every press inside its bounds.

The pure function is unit-tested; the widget itself is checked through the benchmark harness, which builds the real interface headless.

### Hidden while a menu or dialog is open

The editor knows when its menu bar dropdown, the cell's right-click menu, the page's right-click menu, the assets pane's menu or a dialog is open. While any is, the table editor builds the wrapper with the bar marked hidden: the wrapper returns no overlay for it (its tree is still diffed). This avoids the overlay being drawn over a menu and taking its clicks.

### Look

Since the bar is drawn over the block above, it gets an opaque background (the bar color of the window chrome) with a border and rounded corners, and shrinks to its content (no fill spacer: the Delete Table button follows the others after a gap). Its size is not zoomed, as before.

## Risks / Trade-offs

- [The floating bar covers the last line of the block above the table] → It only does so while the cursor is in the table; the gap and the opaque background keep it readable; the block above can still be reached by clicking outside the bar or with the keyboard.
- [While sticky, the bar covers the top visible row of the table, which may hold the cursor] → Only when the table's top is scrolled out of view; scrolling up a little uncovers it. Accepted for keeping the toolbar reachable on long tables.
- [Overlays are asked for on every event and draw] → The wrapper only lays out the bar (a handful of widgets) and works out a rectangle; the benchmark checks no slowdown.
- [An overlay is clipped to the viewport it was given] → The placement only shows the bar wholly inside it.
