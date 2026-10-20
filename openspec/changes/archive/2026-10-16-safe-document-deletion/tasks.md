# Tasks

## 1. Paths and names

- [x] 1.1 Add a check that a path lies strictly inside a directory (normalized, case-folded on Windows, segments with trailing dots/spaces stripped must not be empty, `.` or `..`), with tests
- [x] 1.2 Extend page/directory name validation (blank, `.`, `..`, control characters, Windows device names; directory names ending with a dot or a space), with tests

## 2. Classification and commands

- [x] 2.1 Classify a node as a page or directory node only when its page file / directory passes the containment check
- [x] 2.2 Use the directory name rule in New Directory and when renaming a directory
- [x] 2.3 Delete through an injected remover, checking every path against the document directory again before removing it; keep and report targets whose removal fails (`failed to delete page: ...` / `failed to delete directory: ...`)
- [x] 2.4 Move the files to the Recycle Bin from the application (`trash` crate)

## 3. Guardrail tests

- [x] 3.1 Nodes with tricky texts (empty, blank, `.`, `..`, `...`, `../x`, device names) linking the document directory, its parent or a sibling: F11 never removes the document directory or anything outside it
- [x] 3.2 A failing removal keeps the node and reports it

## 4. Documentation and verification

- [x] 4.1 README: F11 moves files to the Recycle Bin; name rules
- [x] 4.2 fmt, clippy, tests, `openspec validate --all --strict`
- [x] 4.3 Run the app under Xvfb and delete a page with F11
