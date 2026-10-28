# task-management Specification

## Purpose
Reflecting the tasks written in Markdown pages into the mind map: the Next Steps children of page nodes, the ToDo list, the Due Today / Past Due collections, and the checkmark.

## Requirements

### Requirement: Next Steps sync
For every page node, the refresh SHALL make the node's children equal to the first three list items (lines starting with `* ` or `- ` and longer than three characters, without the marker) under the page's `### Next Steps` heading, up to the next heading. A page without the heading SHALL get no children; a page node whose children already match SHALL be left untouched.

#### Scenario: More than three items
- **WHEN** a page's Next Steps lists `* One`, `- Two`, `* x`, `* Three`, `* Four`
- **THEN** the page node's children are `One`, `Two`, `Three`

### Requirement: Next Steps children are kept
A child whose text is still one of the items SHALL be kept, with its icons and attributes, and moved into the items' order; only the other children SHALL be deleted and only the missing items added.

#### Scenario: A checked step stays checked
- **WHEN** the child `step A` of a page node is checked and `* step C` is added to the page's Next Steps after `step A` and `step B`
- **THEN** after the refresh the children are `step A` (still checked, still the same node), `step B` and `step C`

### Requirement: Nodes put under a page node
Adding, pasting or dropping nodes under a page node SHALL show a warning in the status bar that the refresh replaces a page node's children with its Next Steps.

#### Scenario: A node added under a page node
- **WHEN** Tab is pressed on a page node
- **THEN** the status bar warns that the page node's children are replaced by its Next Steps on the next refresh

### Requirement: Stale ToDo items
The refresh SHALL delete the direct children of `root > ToDo` that link a page file `<docDir>/<page>.md` and whose text is no longer `<item> (<page>)` for any of that page's Next Steps items (all of them, not only the first three), or whose page cannot be read. ToDo children linking nothing or linking files outside the document directory, and nodes deeper under ToDo, SHALL be left alone.

#### Scenario: Task removed from its page
- **WHEN** ToDo holds `Gone step (Task)` linking `Task.md` and `Task.md` no longer lists `Gone step`
- **THEN** the refresh deletes that ToDo item, even when it is checked

### Requirement: Due Today and Past Due
The refresh SHALL fill `root > ToDo > Due Today` and `root > ToDo > Past Due` (created when missing) with the Next Steps items of the pages linked by page nodes whose leading date is today, respectively before today, as `<item> (<page>)` nodes linking the page, past-due items ordered by date. Items already among ToDo's direct children SHALL be left out, and a collection whose content already matches SHALL be left untouched.

#### Scenario: Yesterday's task
- **WHEN** page `Task` lists `24/05/09 pay the bill` and today is 2024-05-10
- **THEN** Past Due holds `24/05/09 pay the bill (Task)` linking `Task.md`

#### Scenario: Already in ToDo
- **WHEN** `24/05/09 pay the bill (Task)` is already a direct child of ToDo
- **THEN** Past Due does not list it again

### Requirement: Due dates
A due date SHALL be a leading `yy/MM/dd` (years 2000-2099). A day beyond the end of its month SHALL be clamped to the month's last day; an invalid month or a day outside 1-31 SHALL mean no date.

#### Scenario: February 30th
- **WHEN** an item starts with `23/02/30`
- **THEN** it is due on 2023-02-28

#### Scenario: Month 13
- **WHEN** an item starts with `26/13/40`
- **THEN** it has no due date

### Requirement: Refresh triggers
The refresh SHALL run, together with the search index update, on F12 (reporting a missing document directory) and automatically at most every five minutes while the map is being changed. Pages SHALL be read in the background without blocking the window; nodes deleted or renamed meanwhile SHALL be skipped.

#### Scenario: Editing for a while
- **WHEN** the map has been edited and five minutes have passed since the last refresh
- **THEN** the refresh runs on its own

### Requirement: Add to ToDo (F8)
Add to ToDo SHALL copy the selected nodes, which must all be children of page nodes, to the top of `root > ToDo` in selection order as `<item> (<page>)` linking the page, and select the copies. It SHALL report `ToDo node is missing.` or the nodes that are not items of a page and copy nothing.

#### Scenario: Two items
- **WHEN** items `a` and `b` of page `P` are selected and F8 is pressed
- **THEN** ToDo starts with `a (P)` and `b (P)`, both linking `P.md`

### Requirement: Toggle Checkmark (F7)
Toggle Checkmark SHALL add Freeplane's `button_ok` icon to each selected node that lacks it and remove it from each that has it.

#### Scenario: Toggling twice
- **WHEN** F7 is pressed twice on a node
- **THEN** the node ends up without the checkmark
