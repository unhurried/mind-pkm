## MODIFIED Requirements

### Requirement: Delete (F11)
Delete (F11 or the Delete key) SHALL delete the selected nodes after one confirmation listing them all. The files of page and directory nodes (a page with its assets directory, a directory with its contents) SHALL be deleted with them; any other node SHALL be deleted without touching files. A page or directory whose files cannot be deleted SHALL keep its node and be reported.

#### Scenario: Two targets
- **WHEN** page `P` and directory `D` are selected and F11 is confirmed
- **THEN** the question lists `"P"` and `directory "D"`, and afterwards `P.md`, `P.assets/`, `D/` and both nodes are gone

#### Scenario: A plain node
- **WHEN** a node that links no page or directory is selected and Delete is pressed
- **THEN** the question names it as a node, and once confirmed the node is gone and no file is touched

#### Scenario: Delete asks by default
- **WHEN** Delete is pressed with page `P` selected
- **THEN** a confirmation is shown, and the node and its files stay until it is answered Yes

#### Scenario: Files that cannot be deleted
- **WHEN** F11 is confirmed for page `P` and its files cannot be moved to the Recycle Bin
- **THEN** `failed to delete page: P` is shown, and the node `P` and its files stay

### Requirement: Delete without confirmation
Holding Ctrl with Delete (Ctrl+F11, Ctrl+Delete) SHALL delete the selected nodes as Delete does, without the confirmation. A selected node under another selected node SHALL go with that node, and the root SHALL never be deleted.

#### Scenario: Ctrl skips the question
- **WHEN** Ctrl+Delete (or Ctrl+F11) is pressed with page `P` selected
- **THEN** no confirmation is shown and `P.md`, `P.assets/` and the node are gone

#### Scenario: Root selected
- **WHEN** only the root is selected and Delete or Ctrl+Delete is pressed
- **THEN** nothing is asked and nothing is deleted

### Requirement: Deleted files go to the Recycle Bin
Delete (F11 or the Delete key) SHALL move the files of the deleted pages and directories to the Recycle Bin (the platform's trash) instead of removing them permanently, so that they can be restored from there.

#### Scenario: Restoring a deleted page
- **WHEN** page `P` with `P.assets/` is deleted with F11
- **THEN** `P.md` and `P.assets/` are in the Recycle Bin and can be restored from there

### Requirement: Deletion stays inside the document directory
Delete (F11 or the Delete key) SHALL never remove the document directory itself, nor any file or directory outside it, whatever the texts and links of the selected nodes are.

#### Scenario: Node linking the document directory
- **WHEN** a node whose text is empty links the document directory and Delete is pressed on it
- **THEN** the node is deleted as a plain node and no file or directory is removed

#### Scenario: Node linking the parent of the document directory
- **WHEN** a node `..` links the directory containing the document directory and Delete is pressed on it
- **THEN** the node is deleted as a plain node and no file or directory is removed
