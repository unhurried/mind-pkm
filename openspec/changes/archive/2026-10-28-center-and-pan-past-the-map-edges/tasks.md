# Tasks

## 1. Implementation

- [x] 1.1 Separate margins before and after the map in the layout
- [x] 1.2 Room around the map grown by Esc's centering and by panning, trimmed once out of view
- [x] 1.3 Keep the clamped centering for opening a map, node links and Select Node; remember the scroll position without the room
- [x] 1.4 Tests: Esc centers the root of a small map, the room goes once out of view, panning past the edges; run the app, press Esc on the root and drag empty space
- [x] 1.5 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
