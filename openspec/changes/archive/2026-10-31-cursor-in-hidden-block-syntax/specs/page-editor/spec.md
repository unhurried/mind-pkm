## ADDED Requirements

### Requirement: Cursor in a block's hidden syntax
A cursor placed in a block edited rendered, in syntax the block does not show (a heading's `#`s, a list item's marker, a quote's `>`), SHALL be where the block shows it: at the start of its text. This holds however the cursor got there - an arrow key from a neighbouring block, a press beside the text, leaving selected blocks - so that typed text goes into the block's text.

#### Scenario: Moving into a heading
- **WHEN** the cursor is at the end of the paragraph before the heading `### aaa`, Right is pressed and `bbb ` is typed
- **THEN** the heading reads `### bbb aaa`
