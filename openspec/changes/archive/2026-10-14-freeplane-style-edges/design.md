# Design

## Decisions

- **Underline every node, the root included**: the specs require every node to be drawn the same way; the underline also gives the edges a common anchor (the bottom corners of the node box).
- **Cubic Bézier with control points in the first third of the gap**: leaving the parent horizontally and arriving at the child horizontally, but turning almost vertical close to the parent, so edges to different children diverge after a few pixels and none overlap the child's underline.
