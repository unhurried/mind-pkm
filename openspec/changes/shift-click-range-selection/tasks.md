# Tasks

## 1. Range selection

- [x] 1.1 Add `Doc::select_range` (siblings between, else shown nodes between in document order; the anchor kept while the selection is the range) and call it from Shift+click. Verify with tests in `tests.rs`: siblings without their descendants, nodes under different parents, backwards, a hidden (folded) node left out, consecutive Shift+clicks narrowing from the same anchor, a plain click resetting the anchor.
- [x] 1.2 README: Shift+Click selects the nodes between. Verify by reading the table.

## 2. Integration checks

- [ ] 2.1 Run the app under Xvfb, Shift+click across siblings and look at the selection.
- [ ] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass.
