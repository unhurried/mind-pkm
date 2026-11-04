## 1. Implementation

- [x] 1.1 The node being edited is drawn without the selection background
- [x] 1.2 Ctrl+Right (Edit → Make Child of Node Above) makes the focused node the last child of its previous sibling

## 2. Verification

- [x] 2.1 Test for Ctrl+Right (`tests.rs`); menu, key bindings and README agree
- [x] 2.2 Run the app under Xvfb and look at an edited node
- [x] 2.3 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
