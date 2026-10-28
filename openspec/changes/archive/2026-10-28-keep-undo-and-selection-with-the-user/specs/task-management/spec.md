## MODIFIED Requirements

### Requirement: Next Steps sync
For every page node, the refresh SHALL make the node's children equal to the first three list items (lines starting with `* ` or `- ` and longer than three characters, without the marker) under the page's `### Next Steps` heading, up to the next heading. A page without the heading SHALL get no children; a page node whose children already match SHALL be left untouched. A child whose text is still one of the items SHALL be kept, with its icons and attributes, and moved into the items' order; only the other children SHALL be deleted and only the missing items added. Adding, pasting or dropping nodes under a page node SHALL show a warning in the status bar that the refresh replaces a page node's children with its Next Steps.

#### Scenario: More than three items
- **WHEN** a page's Next Steps lists `* One`, `- Two`, `* x`, `* Three`, `* Four`
- **THEN** the page node's children are `One`, `Two`, `Three`

#### Scenario: A checked step stays checked
- **WHEN** the child `step A` of a page node is checked and `* step C` is added to the page's Next Steps after `step A` and `step B`
- **THEN** after the refresh the children are `step A` (still checked, still the same node), `step B` and `step C`

#### Scenario: A node added under a page node
- **WHEN** Tab is pressed on a page node
- **THEN** the status bar warns that the page node's children are replaced by its Next Steps on the next refresh
