# Design

## Context

The map is an iced `scrollable` holding a stack of pinned node widgets over an edge canvas. Node boxes, link and fold markers are mouse areas or buttons that capture their presses.

## Decisions

- **Where the press is caught**: a `mouse_area` wraps the scrollable. Children handle events first, so a press on a node, marker or scrollbar is captured there and never reaches it; only presses on empty space start a pan.
- **Following the pointer**: while (and only while) a pan is active, a subscription listens to `CursorMoved` and left-button `ButtonReleased` in window coordinates, so the drag keeps working when the pointer leaves the map; it ends when the pointer leaves the window (a release outside the window might never arrive). The first move after the press sets the anchor; each later move scrolls to `start offset - (pointer - anchor)`, clamped at zero (the scrollable clamps the far edge).
- **Cost**: pan messages skip the relayout that other messages trigger, and no cursor subscription exists when not panning, so ordinary mouse movement causes no updates.

## Risks / Trade-offs

- The pointer movement before the first `CursorMoved` after the press is lost - at most one event's worth, not noticeable.
