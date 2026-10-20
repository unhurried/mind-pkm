# Tasks

## 1. Drag and drop

- [x] 1.1 Arm a drag of the selected nodes on a press on a selected non-root node; start it after a small pointer movement
- [x] 1.2 Compute the drop position (before / after / child) from the node under the pointer; reject the dragged nodes and their descendants; unit tests
- [x] 1.3 Move the dragged nodes on release as one undo step; nothing on a no-op drop; unfold a child-drop target
- [x] 1.4 Show the drop position while dragging; cancel on Esc or when the pointer leaves the window

## 2. Verification

- [x] 2.1 Run the app under Xvfb and drag nodes with xdotool (child, before, after, invalid target, click still selects)
- [x] 2.2 README; `openspec validate --all --strict`, fmt, clippy, tests
