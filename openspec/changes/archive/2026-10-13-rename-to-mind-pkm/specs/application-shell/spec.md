## ADDED Requirements

### Requirement: Application name
The application SHALL be named Mind PKM: the window title SHALL end with "Mind PKM", the welcome screen SHALL show it, and the executable SHALL be `mind-pkm.exe`.

#### Scenario: Window title
- **WHEN** the map `notes.mm` is open without unsaved changes
- **THEN** the window title is `notes.mm - Mind PKM`

## MODIFIED Requirements

### Requirement: Per-user settings
The application SHALL keep its settings in `%APPDATA%\mind-pkm\`: the last opened map, and an optional `template.md` that replaces the built-in New Page template. A settings file that is not there SHALL be read from `%APPDATA%\freeplane-pkm\`, where versions under the former name kept it.

#### Scenario: Custom page template
- **WHEN** `%APPDATA%\mind-pkm\template.md` exists
- **THEN** New Page creates pages from it instead of the built-in template

#### Scenario: Settings from the former name
- **WHEN** only `%APPDATA%\freeplane-pkm\` holds the last opened map and a `template.md`
- **THEN** that map is opened on start and New Page uses that template
