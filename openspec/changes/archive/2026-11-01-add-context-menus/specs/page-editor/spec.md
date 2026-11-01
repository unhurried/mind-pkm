## ADDED Requirements

### Requirement: Page context menu
Right-clicking the page SHALL open a menu offering Cut, Copy, Copy as Markdown, Paste, Paste as Plain Text, Select All, Bold, Italic, Underline, Code, Strikethrough, Link and Clear Formatting, in rendered blocks, in the block being edited and in source mode alike.

#### Scenario: Formatting the selection
- **WHEN** `world` is selected in the paragraph `hello world`, the selection is right-clicked and Bold is chosen
- **THEN** the paragraph becomes `hello **world**`

#### Scenario: Closing with Esc
- **WHEN** the page's context menu is open and Esc is pressed
- **THEN** the menu closes and the page is unchanged

### Requirement: Cursor on right-click
A right-click inside the selection SHALL keep it. A right-click anywhere else on the page SHALL first put the cursor where a click there would, without starting to select.

#### Scenario: Right-click outside the selection
- **WHEN** text in one paragraph is selected and another rendered paragraph is right-clicked
- **THEN** the cursor is in the right-clicked paragraph where it was clicked, nothing is selected, and the menu is open
