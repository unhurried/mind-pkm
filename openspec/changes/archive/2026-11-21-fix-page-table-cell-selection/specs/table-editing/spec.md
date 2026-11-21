## MODIFIED Requirements

### Requirement: Selecting cells
Ctrl+L SHALL select the cursor's row and Ctrl+E its cell. Dragging over cells, or Shift+arrow keys at the edge of a cell's text, SHALL select a rectangular range of cells, shown highlighted. Delete or Backspace on a range SHALL empty its cells.

#### Scenario: Clearing a range
- **WHEN** a 2×2 range is selected and Delete is pressed
- **THEN** the four cells are empty and the table keeps its size

#### Scenario: Clicking another cell
- **WHEN** a cell has the caret and another cell is clicked without a selection modifier
- **THEN** the caret moves into the clicked cell and any selection is cleared
- **AND** slight pointer movement within the clicked cell during the press leaves the selection clear
