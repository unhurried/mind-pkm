# Tasks

## 1. Data URL images

- [x] 1.1 Decode a `data:` URL's image in `markdown::images` (media type, base64 / percent-encoding, SVG or raster). Verify with unit tests: base64 PNG, SVG percent-encoded and base64, whitespace and missing padding, a non-image media type, broken base64, a plain path.
- [x] 1.2 Cache the handles in the editor (by `src`, pruned on revision change) and show them in rendered blocks and in the block being edited; an invalid one shows the alt text with a broken-image mark. Verify with editor tests: the same handle on every call, invalid data reported.

## 2. Integration checks

- [x] 2.1 Run the app under Xvfb with a page holding a data URL image and look at it, rendered and while edited.
- [x] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass.
