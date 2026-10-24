## MODIFIED Requirements

### Requirement: Filter commands
F5 or Ctrl+Shift+F SHALL ask for a filter command and apply it: `a` shows all nodes; `t` shows `root > ToDo` with its ancestors and descendants; `d` shows the nodes whose text starts with a `yy/MM/dd` date on or before today, with their ancestors and descendants; any other text shows the nodes containing it, case-insensitively, with their ancestors and descendants. The status bar SHALL show the active filter.

#### Scenario: Keyword filter
- **WHEN** the filter command `stuff` is entered
- **THEN** a node `old stuff` under `Project > Archive` is shown together with `Project` and `Archive`, and unrelated nodes are hidden

#### Scenario: Due items
- **WHEN** the command `d` is entered on 2024-05-10
- **THEN** `24/05/01 old task` is shown and `24/06/01 later` is hidden
