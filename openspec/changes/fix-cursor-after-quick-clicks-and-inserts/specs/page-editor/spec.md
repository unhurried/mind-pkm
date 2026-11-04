## ADDED Requirements

### Requirement: A click ends with its release
A click on the page SHALL end with the release of the button, however soon the release follows the press: moving the pointer afterwards SHALL NOT select text, and a block that is edited as its source later (a paragraph that typing turns into a math block) SHALL keep the cursor where it is rather than where the block was clicked.

#### Scenario: Pointer moved after a quick click
- **WHEN** the paragraph `see link end` (with `link` a link) is clicked quickly, Home is pressed and the pointer is moved over `link`, which is Ctrl+clicked
- **THEN** no text is selected

#### Scenario: A paragraph completed into a math block
- **WHEN** the paragraph `$$` / `\frac{1}{0` / `$$` is clicked quickly before `rac`, End is pressed on its second line and `}X` is typed
- **THEN** the source is `$$` / `\frac{1}{0}X` / `$$`
