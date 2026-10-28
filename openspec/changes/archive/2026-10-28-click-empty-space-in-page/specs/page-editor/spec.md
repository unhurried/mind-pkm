## ADDED Requirements

### Requirement: Clicking beside content
A click on the page SHALL never leave it without a cursor. Beside a block's content it SHALL put the cursor into that block: anywhere in the row of an empty list item (at the start of its text), and to the right of a line (at the end of that line), also when the line holds math or an image. In the gap between two blocks it SHALL put the cursor into the nearer one: at the end of the block above, or at the start of the block below.

#### Scenario: Click in the gap between blocks
- **WHEN** the gap between the list item `two` and the paragraph `After` is clicked in its upper half and `!` is typed
- **THEN** the item becomes `two!`
- **WHEN** the same gap is clicked in its lower half and `?` is typed
- **THEN** the paragraph becomes `?After`

#### Scenario: Click beside an empty list item
- **WHEN** an empty list item `* ` is clicked far to the right of its bullet and `step` is typed
- **THEN** the item becomes `* step`

#### Scenario: Click right of a line with math
- **WHEN** the paragraph `Inline $x^2$ math` is clicked to the right of its text and `!` is typed
- **THEN** the paragraph becomes `Inline $x^2$ math!`
