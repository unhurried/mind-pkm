## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Page and directory nodes
A node SHALL be a page node when it links (directly, or through a node link) the existing file `<docDir>/<text>.md`, and a directory node when it links the existing directory `<docDir>/<text>/`, as long as that file or directory lies inside the document directory and is not the document directory itself. A page's attachments SHALL live in `<docDir>/<text>.assets/`. Pages SHALL be read and written as UTF-8 with a byte-order mark.

#### Scenario: Node linking a missing file
- **WHEN** a node `Plan` links `<docDir>/Plan.md` but the file does not exist
- **THEN** the node is neither a page nor a directory node

#### Scenario: Node resolving to the document directory
- **WHEN** a node whose text is empty or `.` links the document directory
- **THEN** the node is neither a page nor a directory node

### Requirement: Edit (F2) refuses unsafe renames
Edit SHALL refuse an invalid page or directory name, a target page, assets directory or directory that already exists, and a multi-selection (`select a single node to edit.`), changing nothing. When the page was renamed but its assets directory cannot be, it SHALL rename the page back.

#### Scenario: Target exists
- **WHEN** a page is renamed to the name of another existing page
- **THEN** `new page file already exists.` is shown and nothing changes

#### Scenario: Invalid name
- **WHEN** a page is renamed to `..`
- **THEN** `new name includes invalid characters` is shown and nothing changes

### Requirement: Delete (F11)
Delete SHALL delete the selected page and directory nodes together with their files (a page with its assets directory, a directory with its contents) after one confirmation listing them all; a selected node under another selected node goes with that node. When any selected node is neither a page nor a directory node it SHALL delete nothing and report them (`not a page or directory: ...`). A page or directory whose files cannot be deleted SHALL keep its node and be reported.

#### Scenario: Two targets
- **WHEN** page `P` and directory `D` are selected and F11 is confirmed
- **THEN** the question lists `"P"` and `directory "D"`, and afterwards `P.md`, `P.assets/`, `D/` and both nodes are gone

#### Scenario: Files that cannot be deleted
- **WHEN** F11 is confirmed for page `P` and its files cannot be moved to the Recycle Bin
- **THEN** `failed to delete page: P` is shown, and the node `P` and its files stay
