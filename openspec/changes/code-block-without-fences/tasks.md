# Tasks

## 1. Code body

- [ ] 1.1 `CodeBody` in `edit.rs`: code of fenced, indented, empty and unclosed blocks, inside quotes and list items; offsets both ways; writing code back; info string; verify with unit tests

## 2. Editor

- [ ] 2.1 The active code block's text editor holds its code (place, perform, edges, reveal), the highlighter for plain code, the code box view with the language field
- [ ] 2.2 Backspace in empty code, Down after the last block, the language field changing the fence; verify each scenario of "Editing code blocks" in the editor app tests

## 3. Integration

- [ ] 3.1 README and CLAUDE.md; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (also for the Windows target), `cargo test`, `openspec validate --all --strict`; run the app under Xvfb and look at editing code blocks
