# Tasks

## 1. Code font in the editor

- [x] 1.1 Replace the `CODE_FONT` constant with a function that returns BIZ UDGothic when iced's font database has it and the former font otherwise, chosen once; add a unit test that the lookup picks BIZ UDGothic only when a face of that family is in the database, and verify `cargo test` passes
- [x] 1.2 Use the function everywhere the code font was used (rendered code blocks, active code block, highlighter formats, inline code and keys, front matter, math sources) and verify `cargo clippy --all-targets -- -D warnings` passes
- [x] 1.3 Run the app under Xvfb with BIZ UDGothic installed, open a page with Japanese in a code block and inline code, and verify by a screenshot that the code is in BIZ UDGothic and `日本` lines up with two ASCII characters, rendered and while edited

## 2. Export style sheet

- [x] 2.1 List BIZ UDGothic first for `code, kbd, pre` in `assets/export.css` and verify an exported page's HTML carries the new font list (test in the export tests)

## 3. Integration

- [x] 3.1 Verify `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass

## Workflow follow-up

- Archive the change into `openspec/specs/` so the specs describe the new behavior, before the pull request is merged.
