## ADDED Requirements

### Requirement: Status bar messages
Messages in the map window's status bar, such as `saved <file name>` after saving, SHALL name a file by its name only, SHALL be cut off rather than drawn over the rest of the status bar, and SHALL disappear after about ten seconds.

#### Scenario: Saving the map
- **WHEN** the map `notes.mm` is saved with Ctrl+S
- **THEN** the status bar shows `saved notes.mm`, and some ten seconds later no message
