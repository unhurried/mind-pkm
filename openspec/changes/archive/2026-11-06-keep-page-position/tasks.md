# Tasks

## 1. Measuring a few blocks at a time

- [x] 1.1 Keep the heights measured at another width or zoom for guesses, and give the blocks of unknown height widgets a few at a time, nearest to the visible part first, with a message asked for once the page is drawn while some are left (`near.rs`, `after_draw.rs`). Verify with tests in `near.rs` and `perf.rs`: near blocks only at another zoom, measured in batches, as tall as with every block built in the end.

## 2. Keeping the top in place

- [x] 2.1 The page column tells where its widgets are; the page scrolls, right after it is laid out, by as much as the block at the top moved (`keyed_column.rs`, `scroll_watch.rs`, `document.rs`). Verify with tests in `near.rs`, `scroll_watch.rs` and `perf.rs`: a heading at the top stays there through zooming in and out and the measuring after.

## 3. Integration checks

- [x] 3.1 Benchmark scenarios for zooming and resizing a page (`perf.rs`).
- [x] 3.2 Run the app under Xvfb, zoom and resize a long page and look at it.
- [x] 3.3 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass.
