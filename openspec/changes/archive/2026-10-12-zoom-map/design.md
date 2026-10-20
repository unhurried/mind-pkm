# Design

## Context

The map is an iced `scrollable` holding pinned node widgets over an edge canvas. The layout is computed in `update` from measured text sizes (cached per text) and gives the node boxes in the scrollable's content coordinates; dragging nodes, panning and scrolling to a node all work in those coordinates.

## Decisions

- **Zoom by laying out at the zoomed size**, not by scaling a rendered picture (iced has no transform for widget trees): the text is measured and drawn at the zoomed font size, and paddings, icon widths, gaps, markers and edge strokes are multiplied by the zoom. The layout therefore stays in screen pixels, so drag and drop, panning and revealing nodes need no change, and text is drawn sharp at every zoom. The measured text sizes are cached for the current zoom and dropped when it changes.
- **Fixed steps** 50, 60, 70, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300%: predictable, and 100% is always reachable.
- **Ctrl+wheel** is caught by a small wrapper widget around the map's scrollable, which sees events before the scrollable and tracks the modifiers itself, so the scrollable never scrolls on a Ctrl+wheel. Line deltas count one step per notch; pixel deltas (touchpads) are accumulated, 60 px per step, like the scrollable's own line height.
- **Keeping things in place**: keyboard zoom goes through the usual relayout path, which keeps the focused node's position on screen and then reveals it. Wheel zoom relayouts itself and scrolls so that the content point under the pointer stays under it: the layout is the margin plus the content times the zoom, so a point `m` in the scrollable moves to `margin + (m - margin) * new / old`.
- **The zoom belongs to the window**, not to the map: it is not saved to the `.mm` file (Freeplane keeps its zoom out of the map too) and is not part of undo.
- **Key matching**: Ctrl with `+`, `=` or `;` zooms in (`+` is Shift+`=` on US and Shift+`;` on Japanese keyboards, and browsers accept the unshifted key), `-` zooms out, `0` resets; the numeric keypad's `+` / `-` work as well.

## Risks / Trade-offs

- Each zoom step measures every shown node's text again; for very large maps a step may take a moment. Acceptable for an explicit user action.
- Rounding of measured text sizes makes the zoomed layout not exactly proportional, so a wheel zoom may shift the map by a pixel.
