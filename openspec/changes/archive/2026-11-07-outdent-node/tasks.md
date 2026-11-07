## 1. Implementation

- [x] 1.1 Ctrl+Left (Edit → Make Sibling of Parent) makes the focused node the next sibling of its parent

## 2. Verification

- [x] 2.1 Test for Ctrl+Left (`tests.rs`); menu, key bindings and README agree
- [x] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
