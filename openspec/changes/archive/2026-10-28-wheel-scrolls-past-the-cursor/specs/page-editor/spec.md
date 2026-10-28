## ADDED Requirements

### Requirement: Scrolling and the cursor
The page SHALL scroll to keep the cursor in view when the cursor moves, the selection changes or the text is edited. Otherwise the mouse wheel and the scroll bar SHALL scroll the page freely, also taking the cursor out of view.

#### Scenario: Reading on past the cursor
- **WHEN** the cursor is in a paragraph near the top of a long page and the page is scrolled down with the mouse wheel
- **THEN** the page keeps scrolling, past the cursor's line

#### Scenario: Typing after scrolling away
- **WHEN** the cursor's line has been scrolled out of view and a character is typed
- **THEN** the page scrolls back to show the cursor
