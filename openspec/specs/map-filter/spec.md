# map-filter Specification

## Purpose
Showing only part of the mind map by a condition (F5), so that archived material stays out of the way and the task list or due items can be viewed on their own.

## Requirements

### Requirement: Filter commands
F5 SHALL ask for a filter command and apply it: `a` shows all nodes; `t` shows `root > ToDo` with its ancestors and descendants; `d` shows the nodes whose text starts with a `yy/MM/dd` date on or before today, with their ancestors and descendants; any other text shows the nodes containing it, case-insensitively, with their ancestors and descendants. The status bar SHALL show the active filter.

#### Scenario: Keyword filter
- **WHEN** the filter command `stuff` is entered
- **THEN** a node `old stuff` under `Project > Archive` is shown together with `Project` and `Archive`, and unrelated nodes are hidden

#### Scenario: Due items
- **WHEN** the command `d` is entered on 2024-05-10
- **THEN** `24/05/01 old task` is shown and `24/06/01 later` is hidden

### Requirement: Archive filter
The empty filter command SHALL hide every node named `archive` (case-insensitive) and everything under it, and show all other nodes.

#### Scenario: Empty command
- **WHEN** the filter dialog is confirmed with an empty command
- **THEN** `Project > Archive` and its subtree are hidden and everything else is shown

### Requirement: Cancelling keeps the filter
Cancelling the filter dialog SHALL keep the current filter.

#### Scenario: Cancelled
- **WHEN** the filter dialog is cancelled
- **THEN** the previous filter stays in effect

### Requirement: Default filter
A map SHALL open with the archive-hiding filter (the empty command) active.

#### Scenario: Opening a map
- **WHEN** a map with an `archive` node is opened
- **THEN** the archive node and its subtree are hidden

### Requirement: Filter stays current
The filter SHALL be applied to the map as it changes: nodes created, edited or deleted after the filter was set SHALL be shown or hidden by the same condition. The root SHALL always be shown, and a shown node whose parent is hidden SHALL be shown under its nearest shown ancestor.

#### Scenario: New node under an archive
- **WHEN** a node is added under `archive` while the archive-hiding filter is active
- **THEN** the new node is hidden
