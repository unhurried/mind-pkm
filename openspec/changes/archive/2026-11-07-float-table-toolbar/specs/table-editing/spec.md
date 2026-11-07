## ADDED Requirements

### Requirement: Table toolbar placement
The table toolbar SHALL float over the page just above the table without taking room in it: the table and the blocks around it SHALL stay where they are when the cursor goes into or leaves a table. While the top of the table is out of view and some of it is in view, the toolbar SHALL stay at the top of the page's visible part. It SHALL be hidden while a menu or a dialog of the window is open.

#### Scenario: Entering a table moves nothing
- **WHEN** a cell of a rendered table is clicked
- **THEN** the toolbar appears just above the table, and the table and the paragraph below it stay where they were

#### Scenario: Toolbar of a table scrolled partly out of view
- **WHEN** the cursor is in a long table and the page is scrolled so that the table's top is above the visible part while its lower rows are in view
- **THEN** the toolbar is shown at the top of the visible part, over the table

#### Scenario: Hidden while a menu is open
- **WHEN** the cursor is in a table and the Table menu is opened
- **THEN** the toolbar is not shown until the menu closes
