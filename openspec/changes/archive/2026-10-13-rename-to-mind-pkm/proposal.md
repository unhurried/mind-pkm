# Proposal

## Why

The application is no longer tied to Freeplane (it only shares the `.mm` file format), so it gets its own name: Mind PKM.

## What Changes

- The window title and the welcome screen say "Mind PKM".
- The executable is `mind-pkm.exe` and the release archive `mind-pkm-<version>-windows-x64.zip`.
- Settings are kept in `%APPDATA%\mind-pkm\`; a settings file missing there is still read from `%APPDATA%\freeplane-pkm\`, so the last opened map and a custom page template carry over.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `application-shell`: the application name and the settings directory.

## Impact

- `Cargo.toml` (package and binary name), `.github/workflows/build.yml` (release archive), `src/app/` (title, welcome screen, settings directory), README.
