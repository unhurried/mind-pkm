# Proposal

## Why

Code in the page editor is shown in Consolas, which has no Japanese characters, so Japanese in code falls back to whatever font the text engine picks: on Japanese Windows kanji come out in Microsoft YaHei UI (Simplified Chinese glyph shapes) and kana in Yu Gothic, mixed within a line, and full-width characters do not line up with the half-width ones. Japanese comments and strings in code are common in this app's pages.

## What Changes

- The code font (fenced and indented code blocks, inline code, `<kbd>`, and the other text the editor shows in the code font) becomes BIZ UDGothic for half-width and full-width characters alike, so a full-width character is exactly as wide as two half-width ones and columns of mixed Japanese and ASCII line up.
- Where BIZ UDGothic is not installed, the code font stays what it is now (Consolas on Windows, the system's monospace font elsewhere).
- The built-in export style sheet lists BIZ UDGothic first for code in exported HTML and PDF, before its current fonts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `markdown-rendering`: adds a requirement defining the code font (BIZ UDGothic for all characters, the former fonts where it is missing).
- `page-export`: adds a requirement that the built-in export style sheet sets code in BIZ UDGothic first.

## Impact

- Page editor rendering of code and the active code block (`src/app/editor/view/theme.rs` and the places that use the code font).
- `assets/export.css` (built-in export style sheet); a user's own `export.css` is unaffected.
- No new dependency: BIZ UDGothic comes with Windows 10 (version 1809) and later where Japanese is installed; the font is not bundled.
