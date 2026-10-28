# Design

Renaming moves files, which Undo can't take back, so the map and the files can only stay consistent if no undo step names the old files. Two ways were possible: undo the rename on disk too, or make the rename a barrier. Undoing on disk would have to rename the files back, undo the text rewrite inside the page and redo all of it again, with open editors, other applications and the search index in between; failing half way would recreate the inconsistency. The barrier is a few lines and cannot fail.

- The map: after a successful rename the undo and redo steps are dropped. The rename itself is not an undo step, and the map still counts as changed (unsaved, due for a refresh).
- The editor: the history is dropped only when the rename changed the page's text. Every earlier snapshot then contains the old names; the text of a renamed directory's pages is unchanged, so their history stays.
- Delete (F11) has the same kind of problem (files in the Recycle Bin, nodes restored by Undo) and is left as it is; this change is about rename.
