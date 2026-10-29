# Design

## Context

The editor window is a column of menu bar, find bar, body and status bar; the body is a row of the outline (left, fixed width, per window) and the page. The page column's width is computed from the window width minus the outline. Images are copied into the image folder (`images::image_folder`), which by default is the same `<name>.assets/` this pane lists, but the pane always lists `<name>.assets/` (proposal). The editor reports pointer moves over rendered text (`PointerOver` with a `HitInfo` that maps a point to a source offset) and blocks report the pointer entering them; a window-wide release subscription ends drags. Moving files to the Recycle Bin and the document-directory guardrail live in the map's delete (`commands::delete_execute` takes the remover; the app passes `trash`). Editors do not know the map's document directory today.

## Goals / Non-Goals

**Goals:**
- File-system work of the pane UI-free and tested in a temporary directory: reading the tree, renaming, deleting through a given remover, all refusing anything not strictly inside the document directory.
- Rewriting the page's links on rename as edits that splice the source (the page is never regenerated), applied as one undo step.

**Non-Goals:**
- Watching the folder for changes; the pane is refreshed at the moments the spec lists.
- Rewriting links in other files, or reference-style link definitions (`[x]: path`) - only inline links and images of the open page.
- Several items selected or dragged at once; creating folders; moving items between folders.

## Decisions

- **UI-free module `src/pkm/assets.rs`.** `assets_dir_of(file)` (`<stem>.assets` next to the file), `read_tree(dir) -> Option<Vec<Entry>>` (None when missing; entries recursive, folders first, case-insensitive name order), `rename(item, new_name, doc_dir) -> Result<PathBuf, String>` (name checks, taken name, `is_strictly_inside` for both old and new path), `delete(item, doc_dir, remove)`. Errors are the messages shown. Alternative considered: putting it in `markdown/images.rs` - rejected, the pane is about the PKM layout, not image insertion.
- **Whole tree read in the background.** `read_tree` reads all levels in one `blocking` task; expanding a folder is then instant and needs no message round trip. Assets folders are small; if one is huge, the read still never blocks the UI. Alternative: read each folder when expanded - more messages and states for no visible gain.
- **Link rewriting in `src/markdown/`.** A function takes the page text, the page path, the old and new item paths and returns splices: for each inline link and image event (outside code), the destination's byte range in the source is located within the link's range (after `](`, possibly inside `<...>`), resolved with `images::local_path`; when it is the old path or below it, the path segment of the renamed item is replaced with the new name, encoded the way that segment was written (percent-escaped if it was). The editor applies the splices as one edit (one undo step).
- **Pane state per editor.** `Editor` gains `assets: Option<AssetsPane>` (None = hidden, so a new window starts hidden) with the tree, expanded folder paths, the highlighted item, the open context menu and the drag in progress. Shown in `view` as a fixed-width column right of the page; `column_width` subtracts it as it does the outline. `EditorCommand::Assets`, View menu entry "Assets" (checked while shown), Ctrl+Shift+2 matched by the physical Digit2 key with Ctrl+Shift so that layouts printing `@` or `"` on it agree (the `@` character alone already means Code on Japanese keyboards).
- **Double-click** uses iced's `mouse_area::on_double_click`; files open through the existing `Effect::OpenPage` (for `.md`) and `Effect::OpenExternal`.
- **Context menu and dialogs.** Right-click opens a small menu (like the table cell menu) with Rename and Delete, disabled when the editor has no document directory, the item is not strictly inside it, or an editor window has a file inside the item open. Rename uses a new `EditorDialog::Rename` with a text input (the modal layer lets its own widgets take the keys); Delete a new `EditorDialog::DeleteAsset` question. Enter / Escape answer them as the other dialogs.
- **Document directory and remover from the app.** The app hands each editor the map's document directory (`pkm::load_doc_dir`, None without a map or setting) when it opens and whenever the map changes, and performs deletion through an `Effect::DeleteAsset(path)` handled with `trash::delete_all`; app tests use a remover that deletes inside the temporary directory. The "open in an editor" check is also answered by the app, which knows all editors (`Effect` carries the request; the editor keeps the answer per menu).
- **Drag and drop.** Pressing an item and moving the pointer a few pixels starts a drag; the dragged name is drawn pinned at the pointer (a layer of the window stack). The drop target is updated from the messages the page already sends: `PointerOver` gives the source offset under the pointer in rendered text (marked by a caret drawn in that piece of text), `BlockEntered` / pressing beside text gives "after block N" (marked by a line under the block); over the active block, the live text widget reports its hit offset while a drag is on; over a code-like active region the target is "after the block". The window-wide release ends the drag: with a target, the Markdown (from `images::markdown_for`, which already chooses `![]()` for images and `[name]()` otherwise and honours the path options) is spliced in as one edit; released over the pane, bars or outside the page, the target is cleared first, so nothing is inserted. In source mode the link goes at the source editor's cursor.
- **Refreshing.** A `RefreshAssets` task is started on show, on `Focused`, after `Inserted`, after a rename/delete from the pane, after the page's path changes (F2 rename) and from the refresh button; the result replaces the tree, and expanded paths that are no longer folders are dropped.

## Risks / Trade-offs

- [The caret mark inside rendered text needs the text's caret geometry, which the rendered pieces do not draw today] → compute it with the same paragraph as `hit_test` (`grapheme_position`) and pin a thin bar at it in the hovered piece; if a piece cannot give a position, mark the block instead.
- [A huge assets folder makes each refresh read everything] → the read runs in the background and the result only replaces the tree; acceptable for attachment folders.
- [Refusing Rename/Delete for items open in an editor is stricter than the map's F2/F11, which save and follow open editors] → keeps the first version simple and safe; can be relaxed later without changing the rest.
- [Reference-style link definitions are not rewritten on rename] → stated in the spec's scope (links and images); rare in pages, which use inline links.
