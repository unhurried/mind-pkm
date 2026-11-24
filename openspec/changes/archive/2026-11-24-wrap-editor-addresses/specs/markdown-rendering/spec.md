## ADDED Requirements

### Requirement: URL wrapping in the page view

The page view SHALL allow a line to break between adjacent visible characters within a URL. It SHALL fill the available line width with as many of the URL's characters as fit, including when the cursor reveals a Markdown link destination and when the URL is in a table cell. The break positions SHALL leave the Markdown source and link destination intact.

#### Scenario: A URL in a paragraph reaches the margin
- **WHEN** a bare URL or an autolink follows text on a line and extends past the right margin
- **THEN** the first part of the URL fills the remaining width and the next line starts with the first character that did not fit

#### Scenario: A revealed destination is edited
- **WHEN** the cursor reveals an inline link and its destination reaches the margin
- **THEN** the destination wraps between the last character that fits and the next one, including alongside URL punctuation, and the cursor can edit the address on either line

#### Scenario: A long URL is in a table cell
- **WHEN** a table cell displays a URL wider than its text area
- **THEN** the address wraps at character boundaries within that area both while the cell is displayed and while it is edited
