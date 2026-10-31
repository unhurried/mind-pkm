# Tasks

## 1. Implementation

- [x] 1.1 `error_log` module: append an entry (time, version, text) to `mind-pkm-error.log` in the executable's folder, falling back to the temporary folder; rotate past 1 MiB; create the file on the first error only
- [x] 1.2 Panic hook writing message, location and backtrace; `main` installs it and logs an error returned by the application
- [x] 1.3 A failed search index update is logged with its details; the status bar shows `search index update failed (see error log)`
- [x] 1.4 Tests: entry format and appending, no file before the first error, rotation, unwritable folder falls back, the panic report, the index update failure (log and status bar)
- [x] 1.5 README (error log), version 0.14.0
- [x] 1.6 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`; run the app with a forced panic and look at the log
