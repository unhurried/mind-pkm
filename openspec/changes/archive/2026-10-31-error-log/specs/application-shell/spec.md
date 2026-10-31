## ADDED Requirements

### Requirement: Error log
When the application crashes (a panic on any thread), fails to start or run, or fails to update the search index, it SHALL append an entry with the time, the application version and the details to the file `mind-pkm-error.log` in the folder of the executable, or in the system's temporary folder if the executable's folder cannot be written to. Writing the log SHALL never itself show an error or stop the application.

#### Scenario: A crash
- **WHEN** the application panics
- **THEN** `mind-pkm-error.log` next to the executable gets an entry with the panic's message and location

#### Scenario: Folder not writable
- **WHEN** the folder of the executable cannot be written to and the application crashes
- **THEN** the entry is written to `mind-pkm-error.log` in the system's temporary folder

#### Scenario: Search index update fails
- **WHEN** updating the search index fails
- **THEN** the log gets an entry with the error's details, and the status bar shows `search index update failed (see error log)` without them

### Requirement: Size of the error log
The error log file SHALL be created by the first entry only. Once it is larger than 1 MiB, the next entry SHALL keep it as `mind-pkm-error.log.old` (replacing an older one) and start a new `mind-pkm-error.log`.

#### Scenario: No errors
- **WHEN** the application runs without any of these errors
- **THEN** no log file is created

#### Scenario: Large log
- **WHEN** an error occurs while `mind-pkm-error.log` is larger than 1 MiB
- **THEN** it is kept as `mind-pkm-error.log.old` and the entry starts a new `mind-pkm-error.log`
