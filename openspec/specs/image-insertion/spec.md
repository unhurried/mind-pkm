# image-insertion Specification

## Purpose
Putting images into a page from the built-in editor - by drag and drop, a file dialog or the clipboard - and keeping them next to the page in its assets folder, referenced by a relative path.

## Requirements

### Requirement: Ways to insert images
Images SHALL be insertable by typing `![alt](src)`, by dropping one or more image files on the editor window, by Format → Image (Ctrl+Shift+I), which picks one or more files, and by pasting image data or copied image files. Each inserted image SHALL be written as `![](<path>)` at the cursor (after the active block when no text has the cursor), one per line when there are several.

Other files dropped on the window SHALL be inserted as a link to where they are, named by their file name.

#### Scenario: Dropping two images
- **WHEN** `a.png` and `b.jpg` are dropped on the editor of `Trip.md` with the cursor in an empty paragraph
- **THEN** the paragraph becomes `![](Trip.assets/a.png)` and `![](Trip.assets/b.jpg)` on two lines and both files are in `Trip.assets/`

#### Scenario: Dropping a PDF
- **WHEN** `report.pdf` in the document directory is dropped on the editor of `Trip.md` next to it
- **THEN** `[report.pdf](report.pdf)` is inserted and nothing is copied

### Requirement: Copying into the assets folder
An inserted image file SHALL be copied into the image folder - by default the page's `<name>.assets/` next to it, created when missing - unless it already is in that folder; a name taken there SHALL get a number (`a-1.png`). Pasted image data SHALL be saved there as a PNG named `image-<yyyyMMddHHmmss>.png`. Copying SHALL be turned off by the `image_copy=0` setting, in which case the original file is referenced.

#### Scenario: Name already taken
- **WHEN** `a.png` is dropped and `Trip.assets/a.png` already exists with other content
- **THEN** the file is copied as `Trip.assets/a-1.png` and referenced so

#### Scenario: Pasting a screenshot
- **WHEN** a screenshot is on the clipboard and Ctrl+V is pressed
- **THEN** a PNG named `image-` followed by the date and time is saved in the assets folder and inserted

### Requirement: Image folder per page and in general
The `image_folder=` setting SHALL change the image folder for all pages, with `${filename}` standing for the page's name; the front matter key `typora-copy-images-to` SHALL change it for that page. A relative folder SHALL be taken relative to the page's directory.

#### Scenario: Front matter folder
- **WHEN** a page's front matter has `typora-copy-images-to: img` and an image is dropped
- **THEN** the image is copied into `img/` next to the page and referenced as `img/<name>`

### Requirement: How image paths are written
Image paths SHALL be written relative to the page with `/` as the separator, without a `./` prefix and without escaping, so that Edit (F2) renaming the page rewrites them. The `image_dot_slash=1` setting SHALL add `./`, and `image_escape=1` SHALL escape spaces and other characters a URL cannot hold as `%XX`. An image outside the page's drive SHALL be written as an absolute path.

#### Scenario: Default path
- **WHEN** `photo 1.png` is dropped on `My Page.md`
- **THEN** `![](My Page.assets/photo 1.png)` is inserted

#### Scenario: Escaped path
- **WHEN** `image_escape=1` and `image_dot_slash=1` are set and `photo 1.png` is dropped on `My Page.md`
- **THEN** `![](./My%20Page.assets/photo%201.png)` is inserted
