# Design

After every message that may change the layout, the app scrolls by as much as the focused node moved in the layout (so it stays in place on screen) and then, for user actions, scrolls just enough to bring the focused node into view.

For Ctrl+Up / Ctrl+Down the node kept in place is the moved node's parent instead: reordering siblings does not change the height of their block, so the parent and everything outside the parent's children stay where they are, and the focused node is then only scrolled into view when it left it. The command tells `update` through a flag on the app, since Ctrl+Up reaches the command through a key message as well as through the menu.
