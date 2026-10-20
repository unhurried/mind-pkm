# Design

## Settings directory

New settings are written to `%APPDATA%\mind-pkm\`. Reading tries `mind-pkm` first and then `freeplane-pkm`, per file, so users upgrading keep their last opened map and `template.md` without a migration step; the old directory is never written or deleted.

## What keeps the old name

References to the original Freeplane add-on (`unhurried/freeplane-pkm`) and to Freeplane's `.mm` format are about those things, not about this application, and stay as they are. The repository name is unchanged.
