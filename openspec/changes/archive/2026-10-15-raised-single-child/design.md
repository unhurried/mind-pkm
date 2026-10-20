# Design

## Decisions

- **A fixed rise, scaled with the zoom**: 8 px at 100% zoom is enough for the edge to show two rounded corners (the corner radius is limited to half the height difference) without the child looking detached. Layout coordinates are screen pixels, so the rise is multiplied by the zoom like the gaps.
- **Only for a single shown child**: with several children the edges already branch; the rule is about the children shown (after fold and filter), since those are what the edge connects.
- **The child node is raised, not its subtree**: a child's own subtree may put it off its block's center (a single grandchild does), so the child's block is placed so that the child node's center is the rise above the parent's. A chain of single children then climbs in even steps.
- **The subtree's block grows to hold the rise**: the block's height covers the parent and the shifted child block, so a raised child never overlaps the subtree above.
