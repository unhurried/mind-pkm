## ADDED Requirements

### Requirement: Typing with no block active
When no block is active, typing a character, Up, Down, Enter, Home or End SHALL start editing at the cursor, in what the block there shows (in a heading, after its marker). A typed character SHALL be typed into the block; Home and End SHALL move to the start or end of the line.

#### Scenario: Typing right after opening a page
- **WHEN** the page `# Page1` is opened and `abc` is typed without clicking into it
- **THEN** the page reads `# abcPage1` and the heading stays a heading

#### Scenario: End right after opening a page
- **WHEN** the page `# Page1` is opened, End is pressed and ` unsaved` is typed
- **THEN** the page reads `# Page1 unsaved`
