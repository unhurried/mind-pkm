# Tasks

## 1. Node numbers

- [x] 1.1 Let a map keep node numbers from being handed out again (pad its node list), and pad whenever an older copy replaces the current map (undo, redo, cancelling)
- [x] 1.2 Test: a node created after an undo, or after an undo and a redo, gets a number no earlier node had

## 2. Cancelling a new node

- [x] 2.1 Identify undo entries by a serial number returned by the checkpoint; the new node's dialog keeps the serial of its creation
- [x] 2.2 Cancel: go back to the creation entry when it is the latest, else drop it and delete the node from the current map and from the later entries
- [x] 2.3 Tests: cancelling right away restores the map; cancelling after a recorded refresh keeps the refresh, and undo then returns to the map before the refresh without the node

## 3. Verification

- [x] 3.1 fmt, clippy, tests, `openspec validate --all --strict`
- [x] 3.2 Run the app under Xvfb: Tab, Esc leaves the map unchanged
