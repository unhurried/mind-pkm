# Design

## Context

On Windows the executable uses the GUI subsystem: stderr goes nowhere, and a panic ends the process or a background task without a trace. Errors the user is told about in dialogs and the status bar are not recorded either.

## Decisions

- **One small UI-free module, `error_log`.** `write` appends an entry to the log; `install` sets a panic hook that writes the panic's message, location and a backtrace, then calls the previous hook. `main` installs it first and logs an error returned by the application, exiting with code 1.
- **The log sits next to the executable**, as asked. If that folder cannot be written to (e.g. under `Program Files`), the entry goes to the temporary folder; a failure there is ignored. Logging never panics or shows anything itself.
- **Append, rotate once.** Past 1 MiB the log is renamed to `mind-pkm-error.log.old` (replacing an older one) before the next entry, so it cannot grow without bound. A run without errors leaves no file behind.
- **Only crashes, startup failures and failed index updates are logged.** Messages in dialogs and other status bar messages (saved, exported, warnings) are not errors of the application and are left alone.
- **The status bar of a failed index update drops the details** (they can be long file paths and library errors); the log has them.
- **Tests choose the folder.** The functions behind `write` take the folder as a parameter; in tests `write` goes to a folder the test sets for its thread, never next to the test executable.

## Risks / Trade-offs

- A log in the executable's folder is not found by users who cannot write there; the temporary-folder fallback covers that at the cost of a less obvious place.
