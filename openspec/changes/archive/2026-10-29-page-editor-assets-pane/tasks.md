# Tasks

## 1. File-system and link logic (UI-free)

- [x] 1.1 `src/pkm/assets.rs`: `assets_dir_of`, `read_tree` (missing folder → None; folders first, case-insensitive order, recursive); verify with unit tests in a temporary directory
- [x] 1.2 `rename` with the name checks (empty/unchanged, invalid characters and trailing dot/space, taken name, failure reason) and `is_strictly_inside` for old and new paths; `delete` through a given remover with the same guard; verify with unit tests, and extend `delete_never_leaves_the_document_directory` (or add an equivalent guard test) so a path outside or equal to the document directory is never removed or renamed
- [x] 1.3 Link rewriting in `src/markdown/`: splices for inline links and images (not code) whose destination is the renamed item or below it, keeping `./`, `<...>` and `%XX` as written; verify with unit tests (file, folder, escaped, angle-bracketed, code span untouched, other destinations untouched)

## 2. The pane in the editor

- [x] 2.1 `EditorCommand::Assets`, View → Assets with check mark, Ctrl+Shift+2 by physical Digit2; README shortcut table; verify with the `keys.rs` menu/binding tests and a key test for US and Japanese layouts
- [x] 2.2 Pane state per editor (hidden on open), view right of the page, `column_width` accounting for it, `No assets folder`, tree with expand/collapse, highlight on click, refresh button; verify with `tests.rs` tests (toggle per window, contents, expand state kept on refresh)
- [x] 2.3 Background refresh on show, `Focused`, `Inserted`, after a page rename and from the button; verify with `tests.rs` tests driving those messages
- [x] 2.4 Double-click opens: `.md` → `Effect::OpenPage`, other files → `Effect::OpenExternal`, folder → toggle; verify with `tests.rs` tests on the effects

## 3. Rename and delete

- [x] 3.1 The app answers a right-click (`Effect::AssetMenu`) with whether the item may be changed (inside the document directory, holding no open editor's file) and checks again before renaming or deleting; verify with a `tests.rs` test that Rename/Delete are disabled outside the document directory and for an open file
- [x] 3.2 Right-click menu, `EditorDialog::Rename` (text input, Enter/Escape) and rename with link rewriting as one undo step, page left unsaved, errors shown; verify with `tests.rs` tests (renaming a linked image and a folder, Undo, taken name)
- [x] 3.3 `EditorDialog::DeleteAsset` question and deletion through the app's remover (Recycle Bin; a temporary-directory remover in tests); verify with `tests.rs` tests (confirm, cancel)

## 4. Drag and drop

- [x] 4.1 Drag start threshold, dragged name drawn at the pointer, drop target from `PointerOver` / `BlockEntered` / beside-text presses and from the live text widget, cleared over the pane and bars; verify with `tests.rs` tests driving the messages
- [x] 4.2 Drop marks (caret in rendered text, line under a block) and insertion on release as one undo step (`images::markdown_for`), paragraph after the block for beside-text and code-like targets, source mode at the cursor; verify with `tests.rs` tests (image into a paragraph, PDF beside a block, folder, released over the pane, Undo)

## 5. Integration

- [x] 5.1 Run the app under Xvfb, show the pane, expand a folder, rename, delete, drag an image and a file onto the page, and look at screenshots in light and dark themes
- [x] 5.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict` pass

## Workflow follow-up

- Archive the change (`/opsx:archive`) once implemented, so `openspec/specs/page-assets/spec.md` is created.
