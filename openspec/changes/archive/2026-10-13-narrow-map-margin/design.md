# Design

## Decisions

- **Fixed margin in screen pixels**, not scaled by the zoom: it is room to see the map's edge, not part of the map.
- **No extra room for centering**: the large margin existed so that any node could be scrolled to the middle. Centering now clamps at the scroll limits instead; for a node at the map's edge it ends up near the view's edge, which is what users of scrollbars expect.
