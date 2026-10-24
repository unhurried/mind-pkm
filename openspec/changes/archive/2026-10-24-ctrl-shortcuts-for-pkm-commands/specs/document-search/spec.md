## MODIFIED Requirements

### Requirement: Search panel
F6 or Ctrl+F SHALL show the search panel under the map with the keyword box focused (also while the box already has the keyboard), keeping the previous query and results when it is shown again; Esc or Close SHALL hide it. Searching while the index is being updated SHALL work against the index on disk.

#### Scenario: Reopening the panel
- **WHEN** the panel is closed after a search and F6 is pressed again
- **THEN** the panel shows the same query and results

#### Scenario: Ctrl+F in the keyword box
- **WHEN** Ctrl+F is pressed while the keyword box has the keyboard
- **THEN** the keyword box stays focused with its text selected
