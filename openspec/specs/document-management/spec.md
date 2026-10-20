# document-management Specification

## Purpose
Keeping mind map nodes and the files of the document directory in step: the conventions linking page and directory nodes to Markdown files and directories, and the PKM commands that open, create, rename and delete them.

## Requirements

### Requirement: Document directory
The document directory SHALL be read from the map: the text of the first child of `root > config > docDirPath` (a relative path is taken relative to the map file). Commands that need it SHALL report `config node is missing.`, `docDirPath node is missing.` or `document directory is missing.` when the respective part is absent.

#### Scenario: Directory does not exist
- **WHEN** docDirPath names a directory that does not exist and a PKM command needing it is run
- **THEN** the message `document directory is missing.` is shown and nothing changes

### Requirement: Page and directory nodes
A node SHALL be a page node when it links (directly, or through a node link) the existing file `<docDir>/<text>.md`, and a directory node when it links the existing directory `<docDir>/<text>/`, as long as that file or directory lies inside the document directory and is not the document directory itself. A page's attachments SHALL live in `<docDir>/<text>.assets/`. Pages SHALL be read and written as UTF-8 with a byte-order mark.

#### Scenario: Node linking a missing file
- **WHEN** a node `Plan` links `<docDir>/Plan.md` but the file does not exist
- **THEN** the node is neither a page nor a directory node

#### Scenario: Node resolving to the document directory
- **WHEN** a node whose text is empty or `.` links the document directory
- **THEN** the node is neither a page nor a directory node

### Requirement: Open (F1)
Open SHALL open the file of every selected page node and the directory of every selected directory node with the default application, skipping other nodes.

#### Scenario: Mixed selection
- **WHEN** a page node and a plain node are selected and F1 is pressed
- **THEN** only the page's `.md` file is opened

### Requirement: Edit (F2) renames pages and directories
Edit on a page or directory node SHALL ask for a new name and rename the page file and its assets directory (if any), or the directory, together with the node text and link. For a page it SHALL also replace the old file and assets names inside the page (literal text, not patterns) and update the ToDo items copied from the page.

#### Scenario: Renaming a page
- **WHEN** page node `C++ Memo` (with `C++ Memo.assets/`) is renamed to `Rust Memo`
- **THEN** the files become `Rust Memo.md` and `Rust Memo.assets/`, `[assets](C++ Memo.assets)` in the page becomes `[assets](Rust Memo.assets)`, and the ToDo item `step (C++ Memo)` becomes `step (Rust Memo)` linking the new file

### Requirement: Edit (F2) refuses unsafe renames
Edit SHALL refuse an invalid page or directory name, a target page, assets directory or directory that already exists, and a multi-selection (`select a single node to edit.`), changing nothing. When the page was renamed but its assets directory cannot be, it SHALL rename the page back.

#### Scenario: Target exists
- **WHEN** a page is renamed to the name of another existing page
- **THEN** `new page file already exists.` is shown and nothing changes

#### Scenario: Invalid name
- **WHEN** a page is renamed to `..`
- **THEN** `new name includes invalid characters` is shown and nothing changes

### Requirement: Edit (F2) on other nodes
Edit on a node that is neither a page nor a directory node SHALL edit its text; double-clicking a node SHALL do the same as Edit.

#### Scenario: Plain node
- **WHEN** F2 is pressed on a node without a link and a new text is entered
- **THEN** only the node's text changes

### Requirement: New Page (F9)
New Page SHALL, for each selected node, create `<docDir>/<text>.md` from the page template (substituting `${page_assets_name}` with the assets directory name and `${today}` with today's date as `yy/MM/dd`), create the empty `<text>.assets/` directory, link the node to the page and open the page. When the page already exists it SHALL only link the node and report `page file already exists.`; when the assets directory already exists it SHALL report it and create nothing.

#### Scenario: New page
- **WHEN** node `Weekly Review` is selected and F9 is pressed on 2024-05-10
- **THEN** `Weekly Review.md` is created starting with `[assets](Weekly Review.assets)` and containing a `### 24/05/10` journal heading, `Weekly Review.assets/` exists, and the node links the page

### Requirement: New Directory (F10)
New Directory SHALL, for each selected node, create `<docDir>/<text>/`, link the node to it and open it, and report `directory already exists.` when it exists.

#### Scenario: New directory
- **WHEN** node `Projects` is selected and F10 is pressed
- **THEN** `<docDir>/Projects/` is created and the node links it

### Requirement: Delete (F11)
Delete SHALL delete the selected page and directory nodes together with their files (a page with its assets directory, a directory with its contents) after one confirmation listing them all; a selected node under another selected node goes with that node. When any selected node is neither a page nor a directory node it SHALL delete nothing and report them (`not a page or directory: ...`). A page or directory whose files cannot be deleted SHALL keep its node and be reported.

#### Scenario: Two targets
- **WHEN** page `P` and directory `D` are selected and F11 is confirmed
- **THEN** the question lists `"P"` and `directory "D"`, and afterwards `P.md`, `P.assets/`, `D/` and both nodes are gone

#### Scenario: Files that cannot be deleted
- **WHEN** F11 is confirmed for page `P` and its files cannot be moved to the Recycle Bin
- **THEN** `failed to delete page: P` is shown, and the node `P` and its files stay

### Requirement: Add Assets
Add Assets SHALL create the selected page's assets directory and insert `[assets](<name>.assets)` and a blank line at the top of the page. It SHALL refuse a multi-selection, a node that is not a page, and an existing assets directory.

#### Scenario: Adding assets
- **WHEN** Add Assets runs on page `P` whose text is `body`
- **THEN** `P.assets/` exists and the page reads `[assets](P.assets)`, a blank line, `body`

### Requirement: Find Unlinked
Find Unlinked SHALL rebuild `root > unlinked` (created on the right when missing) with three category nodes - `page`, `directory`, `asset` - listing the `.md` files and directories directly in the document directory that no node links, and the `.assets` directories whose page no longer exists, each entry linking its file. The search index directory SHALL never be listed, and links held by the previous `unlinked` subtree SHALL not count as links.

#### Scenario: Running twice
- **WHEN** Find Unlinked is run twice in a row
- **THEN** the second run lists the same entries as the first

### Requirement: Page and directory names
New Page, New Directory and Edit (F2) SHALL reject a page or directory name that is empty or blank, is `.` or `..`, contains a control character or any of `\ / : * ? " < > |`, or is a Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`, in any case, also followed by an extension). A directory name ending with a dot or a space SHALL be rejected as well.

#### Scenario: Device name
- **WHEN** New Page is run on a node `aux`
- **THEN** `page name includes invalid characters` is shown and no file is created

#### Scenario: Directory name ending with a dot
- **WHEN** New Directory is run on a node `Plan.`
- **THEN** `directory name includes invalid characters` is shown and no directory is created

#### Scenario: Page name ending with a dot
- **WHEN** New Page is run on a node `Mr.`
- **THEN** `Mr..md` is created and the node links it

### Requirement: Deleted files go to the Recycle Bin
Delete (F11) SHALL move the files of the deleted pages and directories to the Recycle Bin (the platform's trash) instead of removing them permanently, so that they can be restored from there.

#### Scenario: Restoring a deleted page
- **WHEN** page `P` with `P.assets/` is deleted with F11
- **THEN** `P.md` and `P.assets/` are in the Recycle Bin and can be restored from there

### Requirement: Deletion stays inside the document directory
Delete (F11) SHALL never remove the document directory itself, nor any file or directory outside it, whatever the texts and links of the selected nodes are.

#### Scenario: Node linking the document directory
- **WHEN** a node whose text is empty links the document directory and F11 is pressed on it
- **THEN** it is reported as not a page or directory and nothing is deleted

#### Scenario: Node linking the parent of the document directory
- **WHEN** a node `..` links the directory containing the document directory and F11 is pressed on it
- **THEN** it is reported as not a page or directory and nothing is deleted
