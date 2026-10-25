## ADDED Requirements

### Requirement: Open Externally
The PKM menu SHALL offer Open Externally (without a shortcut), which opens the files of the selected page nodes and the directories of the selected directory nodes with the default application, skipping other nodes, whatever editor Open uses.

#### Scenario: Page in the default application
- **WHEN** a page node is selected and Open Externally is chosen
- **THEN** its `.md` file opens in the application Windows associates with `.md`

### Requirement: Choosing the page editor
The PKM menu SHALL offer Use Built-in Editor, checked by default, which switches whether Open (F1), New Page (F9), search results and link markers open Markdown files in the built-in editor or with the default application. The choice SHALL be remembered across starts.

#### Scenario: Back to the default application
- **WHEN** Use Built-in Editor is unchecked and F1 is pressed on a page node
- **THEN** the page opens with the default application, also after a restart

## MODIFIED Requirements

### Requirement: Open (F1)
Open SHALL open the file of every selected page node in the built-in editor (with the default application when Use Built-in Editor is unchecked) and the directory of every selected directory node with the default application, skipping other nodes.

#### Scenario: Mixed selection
- **WHEN** a page node and a plain node are selected and F1 is pressed
- **THEN** only the page's `.md` file is opened, in an editor window

#### Scenario: Directory node
- **WHEN** a directory node is selected and F1 is pressed
- **THEN** the directory opens in Explorer

### Requirement: New Page (F9)
New Page SHALL, for each selected node, create `<docDir>/<text>.md` from the page template (substituting `${page_assets_name}` with the assets directory name and `${today}` with today's date as `yy/MM/dd`), create the empty `<text>.assets/` directory, link the node to the page and open the page as Open (F1) does. When the page already exists it SHALL only link the node and report `page file already exists.`; when the assets directory already exists it SHALL report it and create nothing.

#### Scenario: New page
- **WHEN** node `Weekly Review` is selected and F9 is pressed on 2024-05-10
- **THEN** `Weekly Review.md` is created starting with `[assets](Weekly Review.assets)` and containing a `### 24/05/10` journal heading, `Weekly Review.assets/` exists, the node links the page, and the page opens in an editor window
