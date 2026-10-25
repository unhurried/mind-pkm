## MODIFIED Requirements

### Requirement: Opening a result
Double-click, or Enter on the selected result, SHALL open a Markdown (`.md`) file in the built-in editor (with the default application when Use Built-in Editor is unchecked) and any other file with its default application, and SHALL report a file that no longer exists instead.

#### Scenario: Deleted since indexing
- **WHEN** a result's file was deleted and the result is opened
- **THEN** `file no longer exists: <path>` is shown in the panel

#### Scenario: Page result
- **WHEN** a result that is a page is double-clicked
- **THEN** the page opens in an editor window
