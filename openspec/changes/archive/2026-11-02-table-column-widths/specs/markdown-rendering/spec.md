## ADDED Requirements

### Requirement: Table column widths
Each column of a table SHALL be as wide as its widest cell on one line when that is within a fixed maximum column width, and the maximum width otherwise, its cells wrapping inside it. A table narrower than the page SHALL be widened to the page's width, each column in proportion to its width. A table wider than the page SHALL keep its column widths and scroll sideways.

#### Scenario: Short and long columns
- **WHEN** a table has a column of one-digit numbers and a column of sentences longer than the maximum column width
- **THEN** the number column is much narrower than the sentence column, the sentences wrap, and the table is as wide as the page

#### Scenario: Short columns only
- **WHEN** a table has two columns whose cells all fit on one line, the second column's cells wider than the first's
- **THEN** the table is as wide as the page and the second column is wider than the first

#### Scenario: Wide table
- **WHEN** a table has so many columns that their widths add up to more than the page's width
- **THEN** the table keeps its column widths, a horizontal scroll bar is shown under it, and scrolling it sideways shows the columns beyond the page's edge while the rest of the page does not move
