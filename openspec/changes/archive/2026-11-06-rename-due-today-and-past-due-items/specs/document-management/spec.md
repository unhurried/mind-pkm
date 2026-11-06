## MODIFIED Requirements

### Requirement: Edit (F2) renames pages and directories
Edit on a page or directory node SHALL ask for a new name and rename the page file and its assets directory (if any), or the directory, together with the node text and link. For a page it SHALL also replace the old file and assets names inside the page (literal text, not patterns) and update the ToDo items copied from the page - the direct children of `root > ToDo` and the items of `ToDo > Due Today` and `ToDo > Past Due` - to name and link the renamed page.

#### Scenario: Renaming a page
- **WHEN** page node `C++ Memo` (with `C++ Memo.assets/`) is renamed to `Rust Memo`
- **THEN** the files become `Rust Memo.md` and `Rust Memo.assets/`, `[assets](C++ Memo.assets)` in the page becomes `[assets](Rust Memo.assets)`, and the ToDo item `step (C++ Memo)` becomes `step (Rust Memo)` linking the new file

#### Scenario: Renaming a page with a past-due step
- **WHEN** Past Due holds `24/05/09 pay the bill (Task)` linking `Task.md` and page node `Task` is renamed to `Chores`
- **THEN** Past Due holds `24/05/09 pay the bill (Chores)` linking `Chores.md`, without waiting for the next refresh
