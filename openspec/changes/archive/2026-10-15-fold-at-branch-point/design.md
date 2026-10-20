# Design

## Decisions

- **Handle at the branch point**: the branch point lies halfway across the gap at the height of the parent's underline (where the edges turn toward the children). A square hit area of the fold marker's size is centered on it, for every node with shown children except the root (the root does not fold). It is an invisible button that shows a white circle with a "−" while hovered, matching the fold marker that replaces it once the node is folded.
- **Under the nodes**: the handle is layered above the edges but below the nodes. A wide sibling of the parent can reach into the hit area; the node keeps its clicks and drags there.
- **Same message as the fold marker**: the click toggles the fold as one undo step, like clicking the child count.
