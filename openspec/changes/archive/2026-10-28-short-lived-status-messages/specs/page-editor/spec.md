## ADDED Requirements

### Requirement: Editor status messages
Messages in an editor window's status bar, such as `saved <file name>` or `2 replaced`, SHALL name a file by its name only, SHALL be cut off rather than drawn over the word and character counts, and SHALL disappear after about ten seconds or with the next edit of the page.

#### Scenario: Saving a page with a long path
- **WHEN** a page deep in the document directory is saved with Ctrl+S
- **THEN** the status bar shows `saved <page>.md` without overlapping the counts

#### Scenario: Editing after saving
- **WHEN** a page is saved and a character is typed
- **THEN** the saved message is gone
