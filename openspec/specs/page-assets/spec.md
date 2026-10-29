# page-assets Specification

## Purpose
The editor's assets pane: seeing the files and folders of a page's assets folder next to the page, opening, renaming and deleting them, and linking to them from the page by drag and drop.

## Requirements

### Requirement: Showing and hiding the assets pane
View → Assets (Ctrl+Shift+2) SHALL show and hide a pane on the right of the editor window, marked with a check in the menu while shown. Each editor window SHALL show or hide its own pane, and the pane SHALL be hidden when a window opens. The page's column SHALL narrow to leave room for the pane and, when it is shown too, the outline.

#### Scenario: Showing the pane
- **WHEN** Ctrl+Shift+2 is pressed in the editor of `Trip.md`
- **THEN** the assets pane appears on the right and View → Assets is checked

#### Scenario: Other windows
- **WHEN** the pane is shown in the editor of `Trip.md` and `Plan.md` is opened
- **THEN** the editor of `Plan.md` opens without the pane and the editor of `Trip.md` keeps it

### Requirement: Contents of the assets pane
The pane SHALL list the `<name>.assets/` folder next to the open file `<name>.md` as a tree: folders before files, each sorted by name ignoring case, folders collapsed until expanded with their arrow. When the folder does not exist, the pane SHALL say `No assets folder`.

#### Scenario: Folder with a subfolder
- **WHEN** `Trip.assets/` holds `map.png`, `Notes.txt` and the folder `raw/` and the pane is shown
- **THEN** the pane lists `raw` (collapsed), then `map.png` and `Notes.txt`

#### Scenario: No assets folder
- **WHEN** the pane is shown for `Plan.md` and `Plan.assets/` does not exist
- **THEN** the pane says `No assets folder`

### Requirement: Keeping the assets pane current
The pane SHALL read the folder again when it is shown, when its window is activated, after files are inserted into the page, after a rename or deletion from the pane, after the open file is renamed, and when its refresh button is pressed. Expanded folders that still exist SHALL stay expanded.

#### Scenario: File added elsewhere
- **WHEN** a file is copied into `Trip.assets/` in Explorer and the editor window of `Trip.md` is activated
- **THEN** the pane lists the new file

#### Scenario: Image pasted
- **WHEN** a screenshot is pasted into `Trip.md` while the pane is shown
- **THEN** the pane lists the saved PNG

### Requirement: Opening assets
Double-clicking a file in the pane SHALL open it: a `.md` file in the editor, any other file with its default application. Double-clicking a folder SHALL expand or collapse it. A single click SHALL highlight the item.

#### Scenario: Opening a PDF
- **WHEN** `report.pdf` is double-clicked in the pane
- **THEN** it opens with the default application for PDF files

### Requirement: Renaming assets
Right-clicking an item SHALL offer Rename, which asks for the new name with the current one filled in. The item SHALL be renamed in its folder, and every link and image in the open page whose destination is the item or inside it SHALL be rewritten to the new name as one undoable change, leaving the page unsaved. A destination SHALL keep how it is written (`./`, `%20` escapes). Links in other files SHALL be left as they are.

#### Scenario: Renaming a linked image
- **WHEN** `Trip.md` holds `![](Trip.assets/map.png)` and `map.png` is renamed to `route.png` in the pane
- **THEN** the file is `Trip.assets/route.png`, the page holds `![](Trip.assets/route.png)` with unsaved changes, and Undo brings back `map.png` in the page's text

#### Scenario: Renaming a folder
- **WHEN** `Trip.md` holds `[raw](./Trip.assets/raw/a%20b.txt)` and the folder `raw` is renamed to `source`
- **THEN** the page holds `[raw](./Trip.assets/source/a%20b.txt)`

### Requirement: Names rejected when renaming
An empty name or the unchanged name SHALL rename nothing. A name containing `\ / : * ? " < > |` or ending with a dot or a space SHALL be rejected with `invalid name: <name>`, a name already taken in the folder with `<name> already exists.`, and a failed rename SHALL be reported with its reason; the file and the page SHALL stay as they were.

#### Scenario: Name taken
- **WHEN** `a.png` is renamed to `b.png` and `b.png` is already in the folder
- **THEN** `b.png already exists.` is shown and nothing is renamed

### Requirement: Deleting assets
Right-clicking an item SHALL offer Delete, which asks `Delete <name>?` and, when confirmed, SHALL move the item (a folder with all it holds) to the Recycle Bin. The page's links to it SHALL be left as they are.

#### Scenario: Deleting a file
- **WHEN** Delete is chosen for `old.png` and the question is confirmed
- **THEN** `old.png` is in the Recycle Bin and no longer listed

#### Scenario: Cancelling
- **WHEN** Delete is chosen for `old.png` and the question is cancelled
- **THEN** `old.png` stays where it is

### Requirement: Assets the pane does not change
Rename and Delete SHALL be offered only for items strictly inside the document directory of the open map, and SHALL NOT be offered for an item holding a file open in an editor window. Nothing outside the document directory, nor the document directory itself, SHALL be renamed or removed from the pane.

#### Scenario: File outside the document directory
- **WHEN** a `.md` file outside the document directory is open and an item of its assets folder is right-clicked
- **THEN** Rename and Delete are shown disabled

#### Scenario: Open in an editor
- **WHEN** `Trip.assets/notes.md` is open in an editor window and right-clicked in the pane of `Trip.md`
- **THEN** Rename and Delete are shown disabled

### Requirement: Linking assets by drag and drop
Dragging an item from the pane onto the page SHALL insert a link to it, written like an inserted image's path (see image-insertion): `![](<path>)` for an image, `[<name>](<path>)` for another file or a folder. While dragging, the name SHALL follow the pointer. A drop outside the page SHALL insert nothing. The insertion SHALL be one undoable change.

#### Scenario: Folder dropped
- **WHEN** the folder `raw` is dragged from the pane of `Trip.md` and dropped into an empty paragraph
- **THEN** the paragraph becomes `[raw](Trip.assets/raw)`

#### Scenario: Dropped back on the pane
- **WHEN** an item is dragged and released over the pane
- **THEN** the page does not change

### Requirement: Where a dragged asset is inserted
While an item is dragged over the page, the place it would go SHALL be marked. A drop into text SHALL insert the link at the text position under the pointer; a drop beside a block's text, or on a code-like block, SHALL insert it as a paragraph after that block (at the end of the block's text, after a space, for a list item or a block in a quote); in source mode it SHALL go at the cursor.

#### Scenario: Image dropped into a paragraph
- **WHEN** `map.png` is dragged from the pane of `Trip.md` and dropped between `see` and `here` in the paragraph `see here`
- **THEN** the paragraph becomes `see ![](Trip.assets/map.png)here`

#### Scenario: PDF dropped beside a block
- **WHEN** `report 1.pdf` is dropped right of the text of the paragraph `Summary`
- **THEN** a paragraph `[report 1.pdf](Trip.assets/report 1.pdf)` follows `Summary`

#### Scenario: Dropped beside a list item
- **WHEN** `a.txt` is dropped right of the text of the list item `- one`
- **THEN** the item becomes `- one [a.txt](Trip.assets/a.txt)`
