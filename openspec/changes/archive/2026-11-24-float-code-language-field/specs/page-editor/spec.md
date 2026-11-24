## MODIFIED Requirements

### Requirement: Editing code blocks
A code block other than a diagram SHALL be edited as its code, in its box with its highlighting: its fences, the indentation of its lines and the prefixes of a quote or list item it is in are not shown, and lines typed get them. A field SHALL show and change its language. The field SHALL float just below the right edge of the box without taking up space in the page. Backspace in an empty code block SHALL turn it into an empty paragraph.

#### Scenario: Editing a code block
- **WHEN** the cursor is in a fenced code block
- **THEN** the block shows its code in its box, without the fences, and its language in a field floating below the box

#### Scenario: Typing in a code block
- **WHEN** the cursor is at the end of `let x = 1;` in the quoted code block `> ```rust` / `> let x = 1;` / `> ```` and Enter and `x` are typed
- **THEN** the block shows `let x = 1;` and `x` without fences, and the source reads `> let x = 1;` and `> x` between the fences

#### Scenario: Changing the language
- **WHEN** `python` is entered in the language field of the code block ` ```rust `
- **THEN** its opening fence reads ` ```python ` and the code is highlighted as Python

#### Scenario: Removing an empty code block
- **WHEN** Backspace is pressed in a code block holding no code
- **THEN** the code block is gone and the cursor is in an empty paragraph in its place

#### Scenario: Entering and leaving a code block
- **WHEN** the cursor enters and then leaves a code block followed by a paragraph
- **THEN** the language field appears and disappears, and the paragraph stays in place

#### Scenario: Scrolling the active code block
- **WHEN** the code block is partly scrolled out of view
- **THEN** the language field stays within the visible page area whenever it fits beside or over the code box

#### Scenario: Opening a menu or dialog
- **WHEN** a window menu or dialog is open while the cursor is in a code block
- **THEN** the language field is hidden
