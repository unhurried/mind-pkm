## MODIFIED Requirements

### Requirement: Following links
Clicking a node's link marker SHALL open a linked Markdown (`.md`) file in the built-in editor (with the default application when Use Built-in Editor is unchecked), open another linked file or directory with its default application, open a URL in the default browser, or, for a node link, select and center the linked node (unfolding its ancestors).

#### Scenario: Node link
- **WHEN** the link marker of a node linking `#ID_123` is clicked
- **THEN** the node with ID `ID_123` is selected and centered

#### Scenario: Page link
- **WHEN** the link marker of a page node is clicked
- **THEN** the page opens in an editor window
