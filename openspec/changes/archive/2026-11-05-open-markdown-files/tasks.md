# Tasks

## 1. Opens report to the window they came from

- [x] 1.1 Make the open path return its error instead of showing it: the map window's callers (F1, New Page, search results, link markers) show it as a map dialog as before, and an editor's followed links and failing default application show it in that editor. Verify with a test in `editor/tests/windows.rs`: Ctrl+click on a link to a missing page shows `failed to open ...` in that editor and no map dialog; existing tests such as `a_page_that_is_not_utf8_is_reported` keep passing.

## 2. File → Open... in the editor

- [x] 2.1 Add the editor command Open... (Ctrl+O) with its File menu entry, the Markdown file dialog starting in the page's folder, the result message and a new effect that opens the chosen file in the built-in editor whatever Use Built-in Editor says. Verify with tests: the chosen file opens in a new window; an open file's window is brought to the front without a second window; a cancelled dialog does nothing; a Shift_JIS file is reported in the requesting editor; the keys.rs check of menu shortcuts against the bindings passes.
- [x] 2.2 README: Ctrl+O in the page editor's shortcut table and Open... among the File menu's commands. Verify that the keys.rs tests still agree with the README.

## 3. Map File → Open picks Markdown files

- [x] 3.1 Give the Open dialog the filters "Mind map or Markdown", "Mind map" and "Markdown" (Save As keeps "Mind map"); a chosen `.md` file opens in an editor window, in the built-in editor, and the map stays as it is. Verify with a test in `tests.rs`: a dirty map, Open, Don't Save, a `.md` file chosen → an editor window holds it, the map is still dirty, and the last map is unchanged.
- [x] 3.2 README: File → Open also opens Markdown files. Verify by reading the section.

## 4. Starting with a Markdown file

- [x] 4.1 In boot, open a `.md` first argument in an editor window without opening the map window, ignoring Use Built-in Editor; on failure, start as without arguments and queue the error in the map window. Verify with tests that boot the app with a `.md` path: one editor window and no map window; with Use Built-in Editor off still the built-in editor; with a missing file the map window's dialog names the file.
- [x] 4.2 Without a map window, end the application when the last editor window closes (after its own unsaved-changes question), without writing the map window's geometry or a map view; record that the application is ending. Verify with tests: closing one of two editors keeps running, closing the second ends; Cancel in the unsaved question keeps it running; ending with the map window still works as before (existing tests, now also asserting that it ends).
- [x] 4.3 README: Installation explains opening `.md` files with Mind PKM from Explorer (Open with / default app) and what a run without a map offers. Verify by reading the section.

## 5. Integration checks

- [x] 5.1 Run the app under Xvfb with a `.md` argument and look at it: only the editor window opens, Ctrl+O opens another file, closing both windows ends the process, and `settings.txt` keeps its `last_file` and `window` lines. Also check the map window's File → Open with a `.md` file.
- [x] 5.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass.

## Workflow follow-up

- Archive the change into `openspec/specs/` (`/opsx:archive`) once implemented and verified.
