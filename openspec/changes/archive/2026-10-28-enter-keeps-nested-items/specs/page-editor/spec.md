## ADDED Requirements

### Requirement: Enter above nested items
Enter at the end of the text of a list item SHALL start the new item with the item's nested items and its other blocks below it, at their level; a blank line between the item's text and those blocks SHALL then go before the new item.

#### Scenario: New item above a nested item
- **WHEN** Enter is pressed at the end of `a` in the list item `a` with the nested item `b`
- **THEN** a new empty item follows `a` at its level, with the cursor in it, and `b` stays nested below the new item

#### Scenario: Nested item after a blank line
- **WHEN** Enter is pressed at the end of `a` in the list item `a` whose nested item `b` follows after a blank line
- **THEN** the blank line comes before the new empty item, and `b` stays nested right below it
