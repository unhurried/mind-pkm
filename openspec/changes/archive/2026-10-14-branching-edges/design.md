# Design

## Decisions

- **Shared trunk and vertical line, rounded corners**: every edge from a parent goes horizontally to a branch point halfway across the gap, vertically to the child's height, and horizontally into the child. The branch point depends only on the parent and the gap (all children of a node start at the same x), so the edges to all children coincide exactly up to where they turn toward their own child.
- **One path, one stroke**: all underlines and edges are stroked as one path, so the parts several edges share are drawn once and are not darkened by antialiasing drawn on top of itself.
- **Corner radius capped at half the height difference**: a child nearly level with its parent gets a gentle S rather than a loop; a level one a straight line.
