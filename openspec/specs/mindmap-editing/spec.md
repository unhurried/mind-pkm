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
Every node, the root included, SHALL use the same font and font size. The application font SHALL be Meiryo UI, falling back to the system fonts where it is missing. Node text wider than 400 px at 100% zoom SHALL wrap; for the nodes below the root's ToDo node (not ToDo itself) the width SHALL be 800 px.

#### Scenario: Root node
- **WHEN** a map is shown
- **THEN** the root's text has the same size as every other node's text

#### Scenario: Long text
- **WHEN** a node outside ToDo has a text that is 600 px wide on one line at 100% zoom
- **THEN** it wraps into lines no wider than 400 px

#### Scenario: Long text below ToDo
- **WHEN** a node below the root's ToDo node has a text that is 600 px wide on one line at 100% zoom
- **THEN** it is shown on one line

### Requirement: Node decorations
A node SHALL show its icons before its text - the checkmark icon as a green check, with the text dimmed - and a link marker after it when it has a link. Every node, the root included, SHALL be drawn the same way: without a frame or background and with the same padding. Selected nodes SHALL be highlighted, the focused one with a stronger border; the node whose text is being edited SHALL have no background.

#### Scenario: Checked node
- **WHEN** a node has Freeplane's `button_ok` icon
- **THEN** it is shown with a green check before its text and the text dimmed

#### Scenario: Root node
- **WHEN** the root node is not selected
- **THEN** it is shown without a frame or background, like any other unselected node

#### Scenario: Node being edited
- **WHEN** the text of the selected node is edited on the map
- **THEN** the node is drawn with its border but without the background of a selected node

### Requirement: Compact spacing
Nodes SHALL be packed closely: at 100% zoom, sibling subtrees 1 px apart, node boxes 1 px of vertical padding and lines 1.2 times the font size high, about 22 px per single-line row.

#### Scenario: List of siblings
- **WHEN** a node has ten single-line children at 100% zoom
- **THEN** they take about 220 px of height

### Requirement: Folding
A folded node SHALL hide its subtree and show the number of its children next to it; clicking that number, or pressing Space with the node selected, SHALL toggle the fold. Clicking the branch point of the edges to a node's children (any node but the root) SHALL fold that node; hovering the branch point SHALL show a marker there. The folded state SHALL be stored in the map (FOLDED attribute).

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
Clicking a node SHALL select it; Ctrl+click SHALL toggle it in the selection. The arrow keys SHALL move the selection: Right to the child nearest the node's vertical center, Left to the parent, Up/Down to the previous/next shown node of the same depth; Home SHALL select the root. The view SHALL scroll to keep the selected node visible, and Esc SHALL center it. When a filter or fold hides a selected node, the selection SHALL move to its nearest shown ancestor.

#### Scenario: Moving down past the last sibling
- **WHEN** the last child of a node is selected and Down is pressed
- **THEN** the selection moves to the first shown node of the same depth under the next parent (a cousin)

#### Scenario: Moving right into a folded node
- **WHEN** a folded node is selected and Right is pressed
- **THEN** the node unfolds and stays selected

### Requirement: Range selection with Shift+click
Shift+click SHALL select the nodes from the anchor to the clicked node, both included, instead of the selection, and focus the clicked node: for siblings, the siblings between them (not their descendants); else every node shown between them in document order (a node before its children). Hidden nodes SHALL NOT be selected. The anchor SHALL be the node focused before the first of consecutive Shift+clicks.

#### Scenario: Shift+click on a sibling
- **WHEN** `A`, `B`, `C` and `D` are siblings, `B` has children, `A` is clicked and `D` is Shift+clicked
- **THEN** `A`, `B`, `C` and `D` are selected and the children of `B` are not

#### Scenario: Shift+click under another parent
- **WHEN** `P` has the children `A` and `B`, `Q` follows `P` with the child `C`, `A` is clicked and `C` is Shift+clicked
- **THEN** `A`, `B`, `Q` and `C` are selected

#### Scenario: Consecutive Shift+clicks
- **WHEN** `A`, `B`, `C` and `D` are siblings, `A` is clicked, `D` is Shift+clicked and then `B` is Shift+clicked
- **THEN** `A` and `B` are selected

### Requirement: Selection after deleting the selected node
When the selected node is deleted, by Delete (F11), Undo or the Next Steps refresh, the selection SHALL move to its next sibling, else its previous sibling, else its parent.

#### Scenario: The selected node is deleted
- **WHEN** the selected directory node `B` between `A` and `C` is deleted with F11
- **THEN** `C` is selected

### Requirement: Structural editing
The application SHALL support: a new child node (Tab or Insert), a new sibling after the selected node (Enter), editing node text (Ctrl+E), deleting the selected nodes (Delete, see Delete (F11)), moving a node up or down among its siblings (Ctrl+Up / Ctrl+Down), and setting or removing a node's link (Ctrl+K). A new node SHALL ask for its text at once, in the node (see In-place text editing); cancelling, or leaving it empty, SHALL remove the node again and keep the map's unsaved state.

#### Scenario: Cancelling a new node
- **WHEN** Tab is pressed on a saved map and Esc is pressed in the new node
- **THEN** no node is added, the previously selected node is selected again, and the map has no unsaved changes

#### Scenario: Cancelling a new node while the map changes
- **WHEN** Tab is pressed, the Next Steps refresh changes the map while the new node is edited, and Esc is pressed
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
Every change to the map, including changes made by PKM commands, SHALL be undoable with Ctrl+Z and redoable with Ctrl+Y (or Ctrl+Shift+Z), at least 100 steps back, and Undo and Redo SHALL select the nodes that were selected at that point. Changes to files in the document directory are not undone; renaming a page or directory with Edit (F2) therefore ends the undo history, so that Undo and Redo never bring back a map naming the files as they were.

#### Scenario: Undo after renaming a page
- **WHEN** a page node is renamed with F2 and Ctrl+Z is pressed
- **THEN** the node, its link and the ToDo items keep the new name, matching the renamed files

### Requirement: Undo and the Next Steps refresh
The Next Steps refresh SHALL NOT be an undo step: its changes SHALL be kept by Undo and Redo, which take back the user's own changes only.

#### Scenario: Undoing a refresh
- **WHEN** a node is pasted, the Next Steps refresh then changes the map, and Ctrl+Z is pressed
- **THEN** the paste is taken back, the refresh's changes stay, and the node selected before the paste is selected

### Requirement: Commands that change nothing
A command that changes nothing, such as New Page or New Directory that fail, SHALL add no undo step and SHALL NOT mark the map as changed.

#### Scenario: A command that fails
- **WHEN** F9 is pressed on a node whose text is not a valid page name
- **THEN** the error is shown, the map is not marked as changed, and Ctrl+Z does not take back anything older

### Requirement: Following links
Clicking a node's link marker SHALL open a linked Markdown (`.md`) file in the page editor (see document-management, Choosing the page editor), open another linked file or directory with its default application, open a URL in the default browser, or, for a node link, select and center the linked node (unfolding its ancestors).

#### Scenario: Node link
- **WHEN** the link marker of a node linking `#ID_123` is clicked
- **THEN** the node with ID `ID_123` is selected and centered

#### Scenario: Page link
- **WHEN** the link marker of a page node is clicked
- **THEN** the page opens in an editor window

### Requirement: Drag to pan
Pressing the left mouse button on an empty part of the map and moving the pointer SHALL scroll the map with the pointer, both horizontally and vertically, until the button is released or the pointer leaves the window. A press on a node, a link marker, a fold marker, a branch point or a scrollbar SHALL keep its own behavior and SHALL NOT start a drag; the drag SHALL NOT change the selection or the map.

#### Scenario: Dragging the map
- **WHEN** the left button is pressed on empty map space and the pointer is moved 100 px to the left and 50 px up
- **THEN** the map content moves 100 px to the left and 50 px up with the pointer

#### Scenario: Press on a node
- **WHEN** the left button is pressed on a node and the pointer is moved
- **THEN** the node is selected and the map does not scroll

#### Scenario: Map edge
- **WHEN** the map is dragged beyond the end of its scrollable area
- **THEN** it stops at the edge

### Requirement: Drag and drop nodes
Pressing the left mouse button on a node and dragging it onto another node SHALL move the node: dropped on the top quarter of the target it SHALL become the target's previous sibling, on the bottom quarter its next sibling, and elsewhere on the target its last child (the target is unfolded). The root SHALL only take children and SHALL NOT be dragged. A move SHALL keep the nodes' IDs and SHALL be one undo step.

#### Scenario: Drop as a child
- **WHEN** node `a` is dragged onto the middle of node `b`
- **THEN** `a` with its subtree becomes the last child of `b`

#### Scenario: Drop as a sibling
- **WHEN** node `a` is dragged onto the top edge of node `c`
- **THEN** `a` is placed right before `c` under `c`'s parent

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

#### Scenario: Drop onto a descendant
- **WHEN** node `a` is dragged onto one of its own children
- **THEN** the map does not change

#### Scenario: Cancelling with Esc
- **WHEN** a node is being dragged and Esc is pressed before the button is released
- **THEN** the map does not change

#### Scenario: Click without moving
- **WHEN** a node is pressed and released without moving the pointer
- **THEN** the node is selected and the map does not change

### Requirement: Zoom steps and shortcuts
The map SHALL be zoomable in fixed steps from 50% to 300%, starting at 100% on every start and kept when another map is opened; the status bar SHALL show the zoom level. View → Zoom In (Ctrl+Plus, also Ctrl+`=`, Ctrl+`;` and the keypad's plus) and Zoom Out (Ctrl+Minus) SHALL change it by one step and Reset Zoom (Ctrl+0) set it to 100%, keeping the selected node in place on screen and in view.

#### Scenario: Zooming in with the keyboard
- **WHEN** Ctrl+Plus is pressed at 100%
- **THEN** the map is shown at the next larger step, its text larger, and the selected node stays where it was on screen

#### Scenario: Zoom level
- **WHEN** the map is zoomed to 150%
- **THEN** the status bar shows 150%

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
The scrollable map area SHALL extend only a narrow margin (40 px, independent of the zoom) beyond the map on every side. Centering a node SHALL scroll only as far as the map area allows.

#### Scenario: Scrolling to the end
- **WHEN** the map is larger than the window and is scrolled all the way down and to the right
- **THEN** the bottom-most and right-most nodes are 40 px from the bottom and right edges of the map area

#### Scenario: Centering the root
- **WHEN** Esc is pressed with the root selected
- **THEN** the root is shown 40 px from the left edge of the map area, vertically as near the middle as scrolling allows

### Requirement: Map context menus
Right-clicking a node SHALL open a menu offering New Child Node, New Sibling Node, Edit Node Text, Cut, Copy, Paste, Fold / Unfold, Set Link and the PKM commands Open, Edit, Toggle Checkmark, Add to ToDo, New Page, New Directory, Delete and Open Externally. Right-clicking empty map space SHALL open a menu offering Paste, Center Selected Node, Zoom In, Zoom Out, Reset Zoom and Update Next Steps and Search Index.

#### Scenario: Right-click on empty space
- **WHEN** empty map space is right-clicked
- **THEN** a menu with Paste and the zoom commands opens and the selection is unchanged

#### Scenario: Running a command from the node menu
- **WHEN** a node is right-clicked and New Child Node is chosen
- **THEN** a child of that node is created as with Tab

### Requirement: Selection on right-click
A right-click on a node outside the selection SHALL select only that node before its menu opens; a right-click on a selected node SHALL keep the selection, so that the menu's commands act on every selected node.

#### Scenario: Right-click on an unselected node
- **WHEN** nodes A and B are selected and node C is right-clicked
- **THEN** only C is selected and its menu is open

#### Scenario: Right-click on a selected node
- **WHEN** nodes A and B are selected, B is right-clicked and Cut is chosen
- **THEN** both A and B are cut

### Requirement: Selection after cancelling a new node
Cancelling a new node, or leaving its text empty, SHALL select again the nodes that were selected before it was created, those still in the map, also when the map changed meanwhile (a refresh); when none of them is left, its parent SHALL be selected.

#### Scenario: Cancelling a new sibling
- **WHEN** the node `a` is selected, Enter is pressed and the new node's text is cancelled with Esc
- **THEN** no node is added and `a` is selected, not its parent

#### Scenario: Leaving a new sibling empty
- **WHEN** the node `a` is selected, Enter is pressed and Enter is pressed again without typing
- **THEN** no node is added and `a` is selected

#### Scenario: Cancelling a new sibling after a refresh
- **WHEN** the page node `Task` is selected, Enter is pressed, the Next Steps refresh changes the map while the new node is edited, and Esc is pressed
- **THEN** the refresh's changes stay and `Task` is selected

### Requirement: In-place text editing
The text of a new node, and the text of a node edited with Edit Node Text (Ctrl+E), Edit (F2) on a node that is neither a page nor a directory node, or a double-click, SHALL be edited in the node itself on the map: the node shows a text box with the node's font, size and wrapping, holding the whole text selected for an existing node. The node SHALL grow and shrink with the text, and the other nodes move to make room.

#### Scenario: Editing a node's text
- **WHEN** Ctrl+E is pressed on node `a`, `b` is typed and Enter is pressed
- **THEN** the node's text is `b`, there is no dialog and Ctrl+Z restores `a`

#### Scenario: A long text wraps
- **WHEN** a text wider than 400 px at 100% zoom is typed into a node
- **THEN** the node wraps the text into lines no wider than 400 px while it is typed

### Requirement: Confirming and cancelling the text
Enter or a click elsewhere on the window SHALL confirm the text being edited in a node, and Esc SHALL cancel it. Confirming a changed text SHALL be one undo step.

#### Scenario: Cancelling an edit
- **WHEN** Ctrl+E is pressed on node `a`, `b` is typed and Esc is pressed
- **THEN** the node's text is still `a` and the map has no unsaved changes

#### Scenario: Leaving a new node's text empty
- **WHEN** Tab is pressed and Enter is pressed without typing
- **THEN** no node is added and the map has no unsaved changes

#### Scenario: Confirming by a click
- **WHEN** a node's text is edited and another node is clicked
- **THEN** the text is confirmed and the clicked node is selected

### Requirement: One line, nothing else acting
The text edited in a node SHALL stay on one line: a line break cannot be entered, and line breaks of pasted text become spaces. While a node's text is edited, the map's keyboard shortcuts SHALL NOT act, and the node SHALL be shown even when the filter hides it.

#### Scenario: A shortcut while editing
- **WHEN** a node's text is edited and Delete is pressed
- **THEN** no node is deleted

#### Scenario: Pasting line breaks
- **WHEN** the text `a\nb` is pasted into a node being edited
- **THEN** the node's text holds `a b`

#### Scenario: A new node under a filter
- **WHEN** a filter is set that the new node's empty text does not match and Tab is pressed
- **THEN** the new node is shown and edited all the same

### Requirement: Indenting a node
Ctrl+Right (Edit → Make Child of Node Above) SHALL move the focused node, with its subtree, to the end of the children of the node above it, its previous sibling, and unfold that node. The move SHALL keep the nodes' IDs and SHALL be one undo step. For the root and for a first child nothing SHALL happen.

#### Scenario: Making a node a child of the node above
- **WHEN** the root has the children `a`, `b` and `c` and Ctrl+Right is pressed on `b`
- **THEN** `b` is the last child of `a`, the root's children are `a` and `c`, and Ctrl+Z restores them

#### Scenario: A first child
- **WHEN** Ctrl+Right is pressed on the first child of a node
- **THEN** nothing changes and the map has no unsaved changes
