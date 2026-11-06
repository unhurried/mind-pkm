## ADDED Requirements

### Requirement: Position kept as the page is laid out anew
When blocks above the visible part of the page change height without being edited - the page zoomed or its window resized, math or a diagram rendered, a block's height measured - the page SHALL scroll with them, so that the block at the top of the visible part stays where it is; after zooming or resizing, as far into that block as before for its new height. Opening, zooming or resizing a long page SHALL NOT wait for every block of the page to be laid out.

#### Scenario: Zooming a scrolled page
- **WHEN** a long page is scrolled down so that the heading `Section 30` is at the top and Ctrl+Shift+= is pressed
- **THEN** the heading `Section 30` stays at the top of the visible part

#### Scenario: Math rendered above the visible part
- **WHEN** a page is scrolled below a display math block, and the math is rendered taller than its source was shown
- **THEN** the text in view stays where it is
