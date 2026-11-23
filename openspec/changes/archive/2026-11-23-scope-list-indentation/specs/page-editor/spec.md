## MODIFIED Requirements

### Requirement: Indenting list items
In the rendered editor, Tab at a caret in a parent item SHALL move its first child after and under that child's next sibling. Tab in a leaf or with its text selected SHALL nest the item under its previous sibling. Tab on a whole-block selection SHALL deepen the selected items in place by one level. Shift+Tab SHALL outdent the active item or the selected range by one level. Each indentation change SHALL be a single undoable edit.

#### Scenario: Tab at a parent
- **WHEN** the list has top-level items `aaa` and `bbb`, with children `ccc` and `ddd` under `bbb` in that order, and Tab is pressed with a caret in `bbb`
- **THEN** `bbb` stays at its original level with the caret, `ddd` stays a child of `bbb`, and `ccc` becomes a child of `ddd`

#### Scenario: Tab on a selected parent and child
- **WHEN** `bbb` and `ccc` in that same list are selected as whole blocks and Tab is pressed
- **THEN** `bbb` becomes a child of `aaa`, `ccc` stays a child of `bbb`, `ddd` stays at its original depth as a sibling of `bbb`, and the selection stays on `bbb` and `ccc`

#### Scenario: Different marker widths
- **WHEN** a selected parent and child are indented under an ordered item with a wider marker
- **THEN** the selected items deepen by one level, and following unselected items keep their original depth

#### Scenario: Indenting task items
- **WHEN** Tab moves one or more task items
- **THEN** each item keeps its checkbox state

#### Scenario: Nesting sibling items together
- **WHEN** one or more consecutive sibling items after the first are selected as whole blocks and Tab is pressed
- **THEN** they are nested at the same level under the item before the selection

#### Scenario: Nesting an item
- **WHEN** the second item of a bullet list is a leaf, has the cursor and Tab is pressed
- **THEN** it becomes a nested item of the first one

#### Scenario: No sibling for the move
- **WHEN** Tab is pressed on a parent whose first child has no next sibling in the same list or a selection without a preceding sibling at its starting level
- **THEN** the list structure and selection stay as they are

#### Scenario: Selection crosses a list boundary
- **WHEN** Tab is pressed on a whole-block selection ending outside the list of the first selected item and its descendants
- **THEN** the list structure and selection stay as they are

#### Scenario: Nesting an empty item
- **WHEN** Enter starts a new item below an item and Tab is pressed, at any depth, before anything is typed
- **THEN** the empty item stays a list item, shown without its marker as typed text, and what is typed next goes into it

#### Scenario: Indenting code or math
- **WHEN** Tab or Shift+Tab is pressed while editing a code or math block
- **THEN** Tab indents and Shift+Tab unindents its lines
