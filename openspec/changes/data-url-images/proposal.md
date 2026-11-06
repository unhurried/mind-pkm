# Proposal

## Why

Pages copied from the web, exported by other editors (Typora can embed images) or written by scripts often hold images inline as `data:` URLs (`![](data:image/png;base64,iVBOR...)`). The page editor treats them like remote URLs: it shows the alt text with a link holding the whole encoded image instead of the picture.

## What Changes

- An image whose `src` is a `data:` URL of an image media type (`image/png`, `image/jpeg`, `image/gif`, `image/bmp`, `image/webp`, `image/x-icon`, `image/tiff`, `image/svg+xml`, ...), base64-encoded or percent-encoded, is shown as the image it holds - in `![alt](...)` and in `<img src>`, rendered and while the block is edited, scaled like an image file.
- A `data:` URL that holds no image or cannot be decoded shows the alt text with a broken-image mark (not the URL).
- The decoded image is kept while the page is open, so it is not decoded again on every redraw.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `markdown-rendering`: Images (`data:` URLs shown).

## Impact

- `src/markdown/images.rs` (decoding a `data:` URL), `src/app/editor/mod.rs` (the decoded images cached), `src/app/editor/view/blocks.rs` and `view/active.rs` (showing them), tests.
- New dependency `base64` (already in the dependency tree). HTML and PDF export keep `data:` URLs as they are, which browsers show already.
