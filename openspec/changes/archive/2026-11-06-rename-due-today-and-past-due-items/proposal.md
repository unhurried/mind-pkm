# Proposal

## Why

Edit (F2) on a page renames the ToDo items copied from it, but only the direct children of `root > ToDo`. The items the refresh collects under `ToDo > Due Today` and `ToDo > Past Due` kept the old page name in their text, `<item> (<old name>)`, and kept linking the old file, which no longer exists: clicking their link marker reported `failed to open <docDir>/<old name>.md (os error 2)` until the next refresh (F12, or the automatic one a few minutes after a change) filled the collections again.

## What Changes

- Renaming a page also renames its items under `ToDo > Due Today` and `ToDo > Past Due`: their text names the new page and they link the renamed file, as the direct ToDo items already do.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document-management`: Edit (F2) on a page updates the page's items in Due Today and Past Due too.

## Impact

- `src/pkm/mod.rs`: `rename_todo_items` also goes through the children of `ToDo > Due Today` and `ToDo > Past Due`.
- Tests in `src/pkm/mod.rs` and `src/pkm/commands.rs`.
