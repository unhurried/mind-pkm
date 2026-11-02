# Tasks

## 1. Column widths

- [x] 1.1 Add the maximum column width constant and a pure function deciding column widths (fit, wrap at the maximum, widen to the page, overflow) with unit tests for each case; verify `cargo test table_widths` passes
- [x] 1.2 Measure a cell's lines as the rendered text shapes them, with a cache in the editor; verify a unit test that a measured line laid out in its measured width stays on one line

## 2. Rendered tables

- [x] 2.1 Lay out rendered tables (also inside quotes and list items) with the computed widths and put tables wider than the page in a horizontal scrollable that reports its offset; verify an editor test on the view's widths (short vs. long columns, short columns widened to the page) and by looking at the app under Xvfb

## 3. The table being edited

- [x] 3.1 Lay out the edited table with the same widths, measuring the cursor's cell from what it shows, and scroll it like the rendered one; verify an editor test that the widths match the rendered table's and that typing widens the column
- [x] 3.2 Keep the scroll position when a scrolled table is clicked, and scroll the cursor's cell into view sideways after moving; verify a unit test of the horizontal offset decision and by Tab through a wide table in the app

## 4. Checks

- [x] 4.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict` and the benchmark; verify all pass

## Workflow follow-up

- Archive the change into the specs (`/opsx:archive`).
