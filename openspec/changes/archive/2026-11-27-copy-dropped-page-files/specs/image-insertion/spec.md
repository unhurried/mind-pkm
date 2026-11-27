## MODIFIED Requirements

### Requirement: Ways to insert images
Images SHALL be insertable by typing `![alt](src)`, by dropping one or more image files on the editor window, by Format → Image (Ctrl+Shift+I), which picks one or more files, and by pasting image data or copied image files. Each inserted image SHALL be written as `![](<path>)` at the cursor (after the active block when no text has the cursor), one per line when there are several.

#### Scenario: Dropping two images
- **WHEN** `a.png` and `b.jpg` are dropped on the editor of `Trip.md` with the cursor in an empty paragraph
- **THEN** the paragraph becomes `![](Trip.assets/a.png)` and `![](Trip.assets/b.jpg)` on two lines and both files are in `Trip.assets/`

## ADDED Requirements

### Requirement: Attaching dropped files
Other files dropped on the editor window SHALL be copied into the page's `<name>.assets/` folder, created when missing. The editor SHALL insert a relative Markdown link named after the stored file. A file already in that folder SHALL be linked in place. A taken name SHALL get a number.

#### Scenario: Dropping a PDF
- **WHEN** `report.pdf` in the document directory is dropped on the editor of `Trip.md` next to it
- **THEN** `report.pdf` is copied to `Trip.assets/` and `[report.pdf](Trip.assets/report.pdf)` is inserted

#### Scenario: Attachment name already taken
- **WHEN** a new `report.pdf` is dropped and `Trip.assets/report.pdf` already exists
- **THEN** the new file is copied as `Trip.assets/report-1.pdf` and the link points to that copy

#### Scenario: Dropping an existing attachment
- **WHEN** `Trip.assets/report.pdf` is dropped on the editor of `Trip.md`
- **THEN** the link points to the existing file in `Trip.assets/`
