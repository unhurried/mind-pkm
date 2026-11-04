## 1. Editing state and layout

- [x] 1.1 The map window keeps the node being edited and its text box; the layout takes the text being edited (and shows the node despite the filter)
- [x] 1.2 New Child / New Sibling / Edit Node Text / Edit (F2) on plain nodes / double-click start editing in the node instead of opening a dialog

## 2. Entering and leaving

- [x] 2.1 Enter and a click elsewhere confirm, Esc cancels; keys of the map do not act meanwhile
- [x] 2.2 One line only: no line breaks entered, pasted ones become spaces
- [x] 2.3 The node shows the text box in place of its text, with the same font, size, wrapping and zoom; the IME is lined up

## 3. Verification

- [x] 3.1 Tests for the flows above (`tests.rs`), update the existing dialog based tests
- [x] 3.2 Run the app under Xvfb and look at editing and a new node
- [x] 3.3 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`, benchmark
