# mindmap-editing Specification

## Purpose
Viewing and editing the mind map in the application window: how the map is laid out and drawn, how nodes are selected and navigated, and the editing operations that do not involve the document directory.

## Requirements

### Requirement: Right-only layout
The map SHALL be laid out with the root on the left and every subtree growing to the right of its parent, children stacked vertically and centered on their parent, joined to it by an edge. A node with only one shown child SHALL instead have that child placed a little higher (8 px at 100% zoom, scaled with the zoom), so that parent and child do not read as one line. All children of the root SHALL be shown on the right, including nodes whose POSITION says left.

#### Scenario: Map made in Freeplane with left-side nodes
- **WHEN** a map whose root has children with `POSITION="left"` is opened
- **THEN** those children are shown on the right of the root like the others

#### Scenario: Single child
- **WHEN** a node has exactly one shown child of the same height
- **THEN** the child is shown slightly above the parent, and the edge between them bends up into the child's underline

### Requirement: Edges
Every node SHALL be underlined by a thin line, and each child SHALL be joined by a thin edge from the right end of its parent's underline to the left end of its own. The edges to a node's children SHALL run as one line halfway to the children, then vertically, and turn with a rounded corner into each child's underline. Shared parts SHALL look like one thin line however many children there are.

#### Scenario: Several children
- **WHEN** a node has three children
- **THEN** one line runs from the right end of its underline, and branches near the children into three, each ending at the underline of one child

#### Scenario: Many children
- **WHEN** a node has twenty children
- **THEN** the line leaving the parent is as thin as an edge to a single child

### Requirement: Fonts
Every node, the root included, SHALL use the same font and font size. The application font SHALL be Meiryo UI, falling back to the system fonts where it is missing. Node text longer than the maximum node width SHALL wrap.

#### Scenario: Root node
- **WHEN** a map is shown
- **THEN** the root's text has the same size as every other node's text

### Requirement: Node decorations
A node SHALL show its icons before its text - the checkmark icon as a green check, with the text dimmed - and a link marker after it when it has a link. Every node, the root included, SHALL be drawn the same way: without a frame or background and with the same padding. Selected nodes SHALL be highlighted, the focused one with a stronger border.

#### Scenario: Checked node
- **WHEN** a node has Freeplane's `button_ok` icon
- **THEN** it is shown with a green check before its text and the text dimmed

#### Scenario: Root node
- **WHEN** the root node is not selected
- **THEN** it is shown without a frame or background, like any other unselected node

### Requirement: Compact spacing
Nodes SHALL be packed closely: at 100% zoom, sibling subtrees 1 px apart, node boxes 1 px of vertical padding and lines 1.2 times the font size high, about 22 px per single-line row.

#### Scenario: List of siblings
- **WHEN** a node has ten single-line children at 100% zoom
- **THEN** they take about 220 px of height

### Requirement: Folding
A folded node SHALL hide its subtree and show the number of its children next to it; clicking that number, or pressing Space with the node selected, SHALL toggle the fold. Clicking the point where the edges from a node other than the root to its children branch SHALL fold the node; hovering that point SHALL show a marker there. The folded state SHALL be stored in the map (FOLDED attribute).

#### Scenario: Unfolding by click
- **WHEN** the child count next to a folded node is clicked
- **THEN** the node unfolds and its children are shown

#### Scenario: Folding by clicking the branch point
- **WHEN** the branch point of the edges between an unfolded node and its children is clicked
- **THEN** the node folds, its children are hidden and the child count is shown next to it

#### Scenario: Undoing a fold by click
- **WHEN** a node is folded by clicking its branch point and Ctrl+Z is pressed
- **THEN** the node is unfolded again

### Requirement: Selection and navigation
Clicking a node SHALL select it; Ctrl+click SHALL toggle it in the selection and Shift+click add it. The arrow keys SHALL move the selection: Right to the child nearest the node's vertical center, Left to the parent, Up/Down to the previous/next shown node of the same depth; Home SHALL select the root. The view SHALL scroll to keep the selected node visible, and Esc SHALL center it. When a filter or fold hides a selected node, the selection SHALL move to its nearest shown ancestor. When the selected node is deleted, by Delete, Delete (F11), Undo or the Next Steps refresh, the selection SHALL move to its next sibling, else its previous sibling, else its parent.

#### Scenario: Moving down past the last sibling
- **WHEN** the last child of a node is selected and Down is pressed
- **THEN** the selection moves to the first shown node of the same depth under the next parent (a cousin)

#### Scenario: Moving right into a folded node
- **WHEN** a folded node is selected and Right is pressed
- **THEN** the node unfolds and stays selected

#### Scenario: The selected node is deleted
- **WHEN** the selected directory node `B` between `A` and `C` is deleted with F11
- **THEN** `C` is selected

### Requirement: Structural editing
The application SHALL support: a new child node (Tab or Insert), a new sibling after the selected node (Enter), editing node text (Ctrl+E), deleting the selected nodes without touching files (Delete), moving a node up or down among its siblings (Ctrl+Up / Ctrl+Down), and setting or removing a node's link (Ctrl+K). A new node SHALL ask for its text at once; cancelling, or leaving the text empty, SHALL remove it again and leave the map's unsaved state as it was.

#### Scenario: Cancelling a new node
- **WHEN** Tab is pressed on a saved map and the text dialog is cancelled
- **THEN** no node is added, the previously selected node is selected again, and the map has no unsaved changes

#### Scenario: Cancelling a new node while the map changes
- **WHEN** Tab is pressed, the Next Steps refresh changes the map while the text dialog is open, and the dialog is cancelled
- **THEN** the new node is gone and the refresh's changes stay, and Ctrl+Z neither brings the new node back nor takes the refresh's changes away

### Requirement: View while moving a node among its siblings
Moving a node up or down among its siblings SHALL NOT scroll the map while the moved node stays in view; when it would leave the view, the map SHALL scroll just enough to keep it in view.

#### Scenario: Moving a node within the view
- **WHEN** Ctrl+Down is pressed on a node whose next sibling is in view
- **THEN** the node takes its sibling's place and the map does not scroll

#### Scenario: Moving a node out of the view
- **WHEN** Ctrl+Down is pressed on a node at the bottom edge of the view
- **THEN** the map scrolls up just enough to keep the moved node in view

### Requirement: Clipboard
Cut and copy (Ctrl+X / Ctrl+C) SHALL put the selected subtrees on an internal clipboard and their text, indented by depth, on the system clipboard. Paste (Ctrl+V) SHALL insert the internally copied subtrees under the selected node when the system clipboard still holds their text, and otherwise SHALL insert the clipboard text one node per line, nesting lines by indentation. Cut-and-pasted nodes SHALL keep their IDs (so node links to them keep working); a copy SHALL get new IDs.

#### Scenario: Pasting text from another application
- **WHEN** the clipboard holds "a\n\tb" from another application and Ctrl+V is pressed
- **THEN** a node `a` with a child `b` is added under the selected node

### Requirement: Undo
Every change to the map, including changes made by PKM commands, SHALL be undoable with Ctrl+Z and redoable with Ctrl+Y (or Ctrl+Shift+Z), at least 100 steps back, and Undo and Redo SHALL select the nodes that were selected at that point. The Next Steps refresh SHALL NOT be an undo step: its changes SHALL be kept by Undo and Redo, which take back the user's own changes only. A command that changes nothing, such as New Page or New Directory that fail, SHALL add no undo step and SHALL NOT mark the map as changed. Changes to files in the document directory are not undone; renaming a page or directory with Edit (F2) therefore ends the undo history, so that Undo and Redo never bring back a map naming the files as they were.

#### Scenario: Undoing a refresh
- **WHEN** a node is pasted, the Next Steps refresh then changes the map, and Ctrl+Z is pressed
- **THEN** the paste is taken back, the refresh's changes stay, and the node selected before the paste is selected

#### Scenario: A command that fails
- **WHEN** F9 is pressed on a node whose text is not a valid page name
- **THEN** the error is shown, the map is not marked as changed, and Ctrl+Z does not take back anything older

#### Scenario: Undo after renaming a page
- **WHEN** a page node is renamed with F2 and Ctrl+Z is pressed
- **THEN** the node, its link and the ToDo items keep the new name, matching the renamed files

### Requirement: Following links
Clicking a node's link marker SHALL open a linked Markdown (`.md`) file in the built-in editor (with the default application when Use Built-in Editor is unchecked), open another linked file or directory with its default application, open a URL in the default browser, or, for a node link, select and center the linked node (unfolding its ancestors).

#### Scenario: Node link
- **WHEN** the link marker of a node linking `#ID_123` is clicked
- **THEN** the node with ID `ID_123` is selected and centered

#### Scenario: Page link
- **WHEN** the link marker of a page node is clicked
- **THEN** the page opens in an editor window

### Requirement: Drag to pan
Pressing the left mouse button on an empty part of the map and moving the pointer SHALL scroll the map with the pointer, both horizontally and vertically, also past the edges of the map and when the whole map fits in the window, until the button is released or the pointer leaves the window. A press on a node, a link marker, a fold marker, a branch point or a scrollbar SHALL keep its own behavior and SHALL NOT start a drag; the drag SHALL NOT change the selection or the map.

#### Scenario: Dragging the map
- **WHEN** the left button is pressed on empty map space and the pointer is moved 100 px to the left and 50 px up
- **THEN** the map content moves 100 px to the left and 50 px up with the pointer

#### Scenario: Map edge
- **WHEN** the whole map fits in the window and empty map space is dragged 100 px to the right and 50 px down
- **THEN** the map moves 100 px to the right and 50 px down with the pointer, beyond the edge of the map area it had

#### Scenario: Press on a node
- **WHEN** the left button is pressed on a node and the pointer is moved
- **THEN** the node is selected and the map does not scroll

### Requirement: Drag and drop nodes
Pressing the left mouse button on a node and dragging it onto another node SHALL move the node: dropped on the top quarter of the target it SHALL become the target's previous sibling, on the bottom quarter its next sibling, and elsewhere on the target its last child (the target is unfolded). The root SHALL only take children and SHALL NOT be dragged. A move SHALL keep the nodes' IDs and SHALL be one undo step.

#### Scenario: Drop as a child
- **WHEN** node `a` is dragged onto the middle of node `b`
- **THEN** `a` with its subtree becomes the last child of `b`

#### Scenario: Drop as a sibling
- **WHEN** node `a` is dragged onto the top edge of node `c`
- **THEN** `a` is placed right before `c` under `c`'s parent

#### Scenario: Drop onto a descendant
- **WHEN** node `a` is dragged onto one of its own children
- **THEN** the map does not change

#### Scenario: Undo
- **WHEN** a node was moved by drag and drop and Ctrl+Z is pressed
- **THEN** the node is back at its previous place

### Requirement: Dragging several nodes
A press without modifiers on a node of a multiple selection SHALL keep the selection; dragging it then SHALL move all selected nodes together, keeping their order in the map, and releasing it without dragging SHALL select just that node.

#### Scenario: Two selected nodes
- **WHEN** nodes `a` and `b` are selected and `b` is dragged onto the middle of node `c`
- **THEN** `a` and `b` become the last children of `c`, `a` before `b`

#### Scenario: Click in a multiple selection
- **WHEN** nodes `a` and `b` are selected and `b` is clicked without moving the pointer
- **THEN** only `b` is selected

### Requirement: Drag and drop feedback and cancelling
While nodes are dragged, the drop position SHALL be shown: an outline around the target for a child, a line along its top or bottom edge for a sibling. A drop onto a dragged node or one of its descendants, or not onto a node, SHALL change nothing; Esc or the pointer leaving the window SHALL cancel the drag. A press without moving the pointer SHALL only select the node.

#### Scenario: Cancelling with Esc
- **WHEN** a node is being dragged and Esc is pressed before the button is released
- **THEN** the map does not change

#### Scenario: Click without moving
- **WHEN** a node is pressed and released without moving the pointer
- **THEN** the node is selected and the map does not change

### Requirement: Zoom shortcuts
The map SHALL be zoomable in fixed steps from 50% to 300%, starting at 100% on every start and kept when another map is opened. Ctrl+Plus (also Ctrl+`=`, Ctrl+`;` and the keypad's plus) SHALL zoom in, Ctrl+Minus SHALL zoom out and Ctrl+0 SHALL reset the zoom to 100%. Zooming with the keyboard SHALL keep the selected node in place on screen and in view.

#### Scenario: Zooming in with the keyboard
- **WHEN** Ctrl+Plus is pressed at 100%
- **THEN** the map is shown at the next larger step, its text larger, and the selected node stays where it was on screen

#### Scenario: Limits
- **WHEN** Ctrl+Minus is pressed at 50%
- **THEN** the zoom stays at 50%

#### Scenario: Reset
- **WHEN** Ctrl+0 is pressed at 200%
- **THEN** the map is shown at 100%

### Requirement: Wheel zoom
Turning the mouse wheel with Ctrl held over the map SHALL zoom in (away from the user) or out (towards the user) one step per notch, keeping the map point under the pointer in place as far as the map's scrollable area allows, and SHALL NOT scroll the map.

#### Scenario: Zooming with the wheel
- **WHEN** the wheel is turned one notch away from the user with Ctrl held while the pointer is over a node
- **THEN** the map is shown one step larger, the same node is still under the pointer, and the map does not scroll

### Requirement: Zooming is a view change
Zooming SHALL scale the whole map - text, icons, markers, spacing and edges - and SHALL leave clicking, dragging nodes and panning working as before. It SHALL NOT change the map, its unsaved state or the undo history.

#### Scenario: Not a change of the map
- **WHEN** a saved map is zoomed
- **THEN** it has no unsaved changes and Undo does not undo the zoom

#### Scenario: Dragging at another zoom
- **WHEN** a node is dragged onto another node at 150%
- **THEN** it becomes that node's last child, as at 100%

### Requirement: Map margin
The scrollable map area SHALL extend a narrow margin (40 px, independent of the zoom) beyond the map on every side, plus the room Esc and panning add where they need it to move the map past its edges; that room SHALL go again once it is scrolled out of view. Esc SHALL center the selected node wherever it is; centering a node when a map is opened, a node link is followed or a search result is selected SHALL scroll only as far as the map area allows.

#### Scenario: Scrolling to the end
- **WHEN** the map is larger than the window and is scrolled all the way down and to the right
- **THEN** the bottom-most and right-most nodes are 40 px from the bottom and right edges of the map area

#### Scenario: Centering the root
- **WHEN** a small map sits at the top left of the window and Esc is pressed with the root selected
- **THEN** the root moves to the center of the window
