## ADDED Requirements

### Requirement: Moving list items
Ctrl+Up and Ctrl+Down in a list item (bullet, ordered or task item, at any depth, inside quotes and list items too) SHALL swap the item, together with its nested items and the blocks under it, with the previous and next item of the same list, keeping the cursor where it was in the item's text. The items of an ordered list SHALL keep the numbers in their places. Where the item has no such neighbor, and outside list items, the keys SHALL change nothing.

#### Scenario: Moving a task up
- **WHEN** the cursor is in `- [ ] b` of `- [x] a` / `- [ ] b` and Ctrl+Up is pressed
- **THEN** the list is `- [ ] b` / `- [x] a`, and the cursor is still in `b`

#### Scenario: An item with nested items
- **WHEN** the cursor is in `- a` of `- a` / `  - a1` / `- b` and Ctrl+Down is pressed
- **THEN** the list is `- b` / `- a` / `  - a1`

#### Scenario: A nested item stays under its parent
- **WHEN** the cursor is in the first of the nested items of `- a`, which is followed by `- b`, and Ctrl+Up is pressed
- **THEN** nothing changes

#### Scenario: An ordered list keeps its numbers
- **WHEN** the cursor is in `2. b` of `1. a` / `2. b` and Ctrl+Up is pressed
- **THEN** the list is `1. b` / `2. a`

### Requirement: Move item commands
The Paragraph menu SHALL offer Move Item Up (Ctrl+Up) and Move Item Down (Ctrl+Down). In the source mode they SHALL move the list item at the cursor alike.

#### Scenario: Moving in the source mode
- **WHEN** the source mode is on, the cursor is in `- b` of `- a` / `- b` and Ctrl+Up is pressed
- **THEN** the page is `- b` / `- a`
