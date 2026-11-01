## 1. Rendering

- [x] 1.1 Heading sizes as 2, 1.5, 1.25, 1.125, 1 and 1 times the body size
- [x] 1.2 One frame per level (rule, band, chips) around a heading's text, used by rendered blocks, nested headings and the heading being edited
- [x] 1.3 Level 6 in the muted color

## 2. Verification

- [x] 2.1 Run the app and look at rendered and active headings
- [x] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
