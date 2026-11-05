# Proposal

## Why

The page editor is a full Markdown editor, but it can only be reached through a page node of a map: a `.md` file opened with Mind PKM from Explorer (or given on the command line) is taken for a map and fails with `failed to open the map`. Users want to use the editor for any Markdown file, on its own, without a map.

## What Changes

- Starting the application with a `.md` file as its first command-line argument (so `.md` files can be opened with it from Explorer) opens that file in an editor window only, without the map window, in the built-in editor whatever Use Built-in Editor says. The last opened map and the remembered map window are left as they were. A file that cannot be opened is reported in the map window of a normal start instead.
- Without a map window, closing the last editor window ends the application.
- The editor's File menu gets Open... (Ctrl+O): it picks a `.md` file, starting in the folder of the window's file, and opens it in an editor window of its own (or brings its window to the front).
- The map window's File → Open (and the welcome screen's Open) also picks `.md` files; a chosen `.md` file opens in an editor window and the open map stays as it is.
- Errors of opening a file from an editor window (File → Open..., following a link) are shown in that editor window instead of the map window, which may not exist.

Out of scope: handing a file to an already running instance (each file opened from Explorer starts a process of its own, as for maps), File → New and Save As in the editor, dropping `.md` files on the map window, registering the file association (the README explains how to set it with Windows' Open with).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `application-shell`: Opening maps (the first argument may be a Markdown file; File → Open also picks `.md` files), Map window and editor windows (a run without a map window, ended by closing its last editor window); a new requirement for starting with a Markdown file.
- `page-editor`: a new requirement for opening files from an editor window (File → Open..., and where its errors are shown).

## Impact

- `src/app/mod.rs` (`App::boot` dispatching the first argument; whether the map window is open), `src/app/file.rs` (the Open dialog's filters, a chosen `.md` file, exiting without a map window), `src/app/editors.rs` (opening from an editor with errors shown there; ending the application after the last editor window closes), `src/app/editor/` (`EditorCommand::Open`, its menu entry, Ctrl+O, the file dialog, a new `Effect`), tests in `src/app/tests.rs` and `src/app/editor/tests/`.
- README (Installation: opening `.md` files from Explorer; the page editor's shortcuts).
- No new dependencies; the `.mm` reader/writer and the document directory are untouched.
