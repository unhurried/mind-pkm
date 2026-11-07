# Tasks

## 1. Floating widget

- [x] 1.1 Add the pure placement function (table bounds, bar size, viewport -> the bar's top-left or nothing: above the table, pushed down to the viewport's top but not below the table's bottom, hidden unless wholly in view) with unit tests for each case; verify with `cargo test floating`
- [x] 1.2 Add the wrapper widget (`src/app/floating.rs`): layout of the content only, the bar laid out without taking room, base events and drawing for the content, the bar as an overlay at the placement (none when hidden), `operate` on both; verify it builds with `cargo clippy --all-targets -- -D warnings`

## 2. Table editor

- [x] 2.1 Build the table being edited as the wrapper around the grid with the toolbar as its bar (opaque background, border, shrunk to its content), hidden while a menu, right-click menu, assets menu or dialog of the window is open; verify the existing table tests pass (`cargo test editor::tests::tables`)
- [x] 2.2 Add benchmark-harness tests: clicking a cell of a rendered table leaves the table's cells and the block below in place and shows the toolbar above it; a click on a toolbar button (Center) reaches it and not the paragraph under it; with the table's top scrolled out of view the toolbar is at the top of the page's visible part; with a menu open no toolbar is shown; verify with `cargo test --lib perf::`
- [x] 2.3 Update the README's table paragraph (the toolbar floats above the table) and CLAUDE.md's structure notes for the new widget; verify `cargo test keys` still passes

## 3. Integration

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict`, and run the benchmark to compare with before
- [x] 3.2 Run the app under Xvfb, open a page with a table, click a cell and compare screenshots before and after the click (the table does not move, the toolbar floats above it)

## Workflow follow-up

- Archive the change (`/opsx:archive`) so `openspec/specs/table-editing/spec.md` carries the new requirement text, then open the pull request.
