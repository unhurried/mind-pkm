## 1. Style sheet

- [x] 1.1 Heading sizes, colors and line height as in the page view
- [x] 1.2 Rule (level 1), band with chip (level 2), chip and rule (level 3), muted chip (level 4)
- [x] 1.3 Keep the band and chips when printing; keep a heading with the text after it

## 2. Verification

- [x] 2.1 Export a page with all heading levels to HTML and PDF and compare with the page view
- [x] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict`
