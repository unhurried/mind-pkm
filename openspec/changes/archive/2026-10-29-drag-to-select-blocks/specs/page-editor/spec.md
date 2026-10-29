## MODIFIED Requirements

### Requirement: Selecting several blocks
Outside source mode, a selection reaching beyond a block SHALL select whole blocks: Shift+Up/Down at the first/last line of a block, Shift+click on another block, dragging the mouse from a block onto another, and Ctrl+A pressed a second time (the first selects the active block's text) SHALL select a range of blocks, shown highlighted. Copy, Cut, Delete and typing SHALL act on all selected blocks.

#### Scenario: Deleting three blocks
- **WHEN** three paragraphs are selected with Shift+Down and Delete is pressed
- **THEN** the three paragraphs are removed and the cursor is in the block that followed them

#### Scenario: Dragging over blocks
- **WHEN** the mouse button is pressed in the text of the second of four paragraphs and the pointer is dragged onto the fourth
- **THEN** the second to fourth paragraphs are selected, and moving the pointer back onto the third leaves the second and third selected until the button is released
