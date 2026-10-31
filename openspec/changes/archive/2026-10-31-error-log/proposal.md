# Proposal

## Why

The application has no console window, so when it crashes or fails to start, nothing says why. A failed search index update only shows its full error text in the status bar for about ten seconds.

## What Changes

- Crashes (a panic on any thread) and a failure of the application to start or run are written to an error log file, `mind-pkm-error.log`, in the folder of the executable. If that folder cannot be written to, the log goes to the system's temporary folder instead.
- A failed search index update is written to the same log with its details. The status bar shows only `search index update failed (see error log)`.
- Entries are appended with the time and the application version. The file is created by the first error, and the previous log is kept as `mind-pkm-error.log.old` when it grows past 1 MiB.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `application-shell`: new requirement for the error log, and the status bar message of a failed search index update.

## Impact

- New `src/error_log.rs`, `src/main.rs` (panic hook, startup failure), `src/app/search_panel.rs` (index update failure).
- README, version bump to 0.14.0.
