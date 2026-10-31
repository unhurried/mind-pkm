## ADDED Requirements

### Requirement: Clicking in the margins
A click in the margin left or right of the page's column SHALL put the cursor at the start of the line at the height of the click, as a click at the left edge of that line would.

#### Scenario: Click in the margin beside a line
- **WHEN** the margin right of the paragraph `hello world` is clicked at the height of its line and `!` is typed
- **THEN** the paragraph becomes `!hello world`
- **WHEN** the margin left of a line of a paragraph of several lines is clicked and `?` is typed
- **THEN** `?` is inserted at the start of that line
