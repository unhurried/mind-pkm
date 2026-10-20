# Design

## Context

A node's page file is `<docDir>/<text>.md` and its directory `<docDir>/<text>/`, built by joining the node text to the document directory; classification compares that path with the node's link after normalizing both. An empty text therefore yields the document directory itself, `..` its parent, and on Windows a name ending with dots or spaces is resolved with them stripped (`...` is the document directory again). Delete removed the computed paths with `remove_file` / `remove_dir_all`, ignoring a failed page removal.

## Goals / Non-Goals

**Goals:**
- No path the PKM commands delete can be the document directory or lie outside it, by construction and by a second check right before deleting.
- Deleted documents can be restored from the Recycle Bin.
- Names the file system would resolve to something else cannot be created.

**Non-Goals:**
- Undoing a deletion from within the application (the Recycle Bin restores the files, Ctrl+Z the nodes).
- Changing which existing nodes count as pages or directories, other than those resolving to the document directory or outside it.

## Decisions

- **Containment check**: a path lies strictly inside a directory when, after normalization (and case folding on Windows, as for every path comparison), it starts with the directory and has at least one more segment, and none of those segments is empty, `.` or `..` once trailing dots and spaces are stripped (the way Windows resolves them). The check is pure string arithmetic on normalized paths, so it is testable on every platform and behaves the same on all of them.
- **Classification**: a node is a page or directory node only if its page file / directory passes the containment check. Nested names such as `sub/page`, which some maps may use, keep working because they stay inside.
- **Delete runs the check again** on every path it is about to remove, so a target computed earlier (the confirmation dialog is answered later) or by a future caller cannot slip through; a failing target keeps its node and is reported.
- **Recycle Bin via the `trash` crate**: on Windows it uses the shell's `IFileOperation` with "allow undo", so the files go to the Recycle Bin; where a location has none (e.g. a network share), Windows warns before deleting permanently, and declining counts as a failure. COM is initialized single-threaded, which matches the UI thread (winit initializes OLE there). On Linux it uses the freedesktop trash, which is what development under Linux exercises.
- **Removal is injected**: the deletion takes a function that removes the paths of one target, so tests delete permanently inside a temporary directory and never touch the real trash; the application passes the Recycle Bin. One target's existing paths (page + assets, or the directory) are removed in one call; when it fails the node stays and the target is reported as `failed to delete page: ...` / `failed to delete directory: ...` (the latter the add-on's wording).
- **Name rules**: one validation for both kinds - blank, `.`, `..`, control characters, the reserved characters, and Windows device names (the part before the first dot, trailing spaces ignored, case-insensitive). Directory names additionally must not end with a dot or a space; page names may, because their files end in `.md` / `.assets`. The messages stay the add-on's ("... includes invalid characters").

## Risks / Trade-offs

- A document directory on a volume without a usable trash (some network shares, a Linux mount without a trash directory) can no longer be cleaned up with F11 without the Windows prompt or a reported failure; deleting by hand remains possible.
- A node that links the document directory or a path outside it through a page-like name is no longer offered for rename or delete; such nodes were never real pages.
