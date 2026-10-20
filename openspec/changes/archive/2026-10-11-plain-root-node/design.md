# Design

## Decisions

- Drop the root-specific padding and style: node size measurement and rendering use the normal node padding and the transparent, frameless style for every node. The root keeps no special case in the view at all.
