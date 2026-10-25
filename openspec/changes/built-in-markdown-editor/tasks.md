# Tasks

## 1. Dependencies and module skeleton

- [x] 1.1 Add `pulldown-cmark`, `latex-rust`, `mermaid-rs-renderer` (no default features), `syntect` + `two-face` (fancy-regex), `arboard`, `emojis`, `htmd`, `image` (selected codecs), `unicode-width`, `unicode-segmentation`, and iced's `image-without-codecs` and `svg` features; create `src/markdown/` and `src/app/editor/`; verify `cargo build` and `cargo build --release --target x86_64-pc-windows-gnu` succeed

## 2. Markdown core (`src/markdown/`)

- [x] 2.1 Block segmentation with byte ranges (flattened list items, continuation and `Other` blocks, front matter, `[toc]`, footnote definitions) and lenient link destinations with offset mapping; verify with unit tests and a property test that every non-blank byte is in exactly one block
- [x] 2.2 Inline model: styled runs with source ranges, links (inline, reference, heading anchors, autolinks, bare URLs), highlight, emoji shortcodes, sub/superscript, math, images, footnote references, breaks, supported inline HTML and placeholders; verify with unit tests per element
- [x] 2.3 Page text I/O: BOM and line-ending detection and restoration, UTF-8 check with `failed to open <path>: <reason>`; verify a CRLF+BOM page round-trips byte for byte with one word changed
- [x] 2.4 Edit rules as pure functions: Enter/Shift+Enter (paragraph, heading, list continuation and exit, quote, code indentation, fence/`$$`/front matter/table-header conversion), Backspace/Delete at markers and block edges, Tab/Shift+Tab on list items; verify each scenario of page-editor's editing requirements in unit tests, plus a property test that an edit inside a block leaves the text outside it unchanged
- [x] 2.5 Format and paragraph operations: bold/italic/underline/code/strikethrough toggles (word at cursor), link insertion, heading level set/raise/lower, quote/ordered/bullet toggles over blocks, clear formatting, auto-pair rules, task toggle, selection helpers (word, line, styled span); verify with unit tests
- [x] 2.6 Undo history with typing coalescing, find/replace (case option, replace all), word/character counts (CJK as words, markers excluded), heading outline and anchors; verify with unit tests

## 3. Tables model

- [x] 3.1 Table parse/serialize (alignment, escaped pipes, `<br>`, padded columns by display width) and operations (insert/delete/move rows and columns, align, resize, clear range, paste TSV/HTML grid with growth, copy as TSV/HTML/Markdown); verify with unit tests and a parse∘serialize property test

## 4. Multi-window application and opening pages

- [x] 4.1 Switch to `iced::daemon`: open the map window in `boot`, window ids in key/resize/move/close/file-drop messages, per-window title/view/theme; verify existing app tests pass and a test routes keys by window id
- [x] 4.2 `open_page` entry used by F1, New Page, search results and link markers: built-in editor for `.md` (focusing an already open window), default application otherwise or when the editor is turned off; PKM menu Open Externally and Use Built-in Editor (remembered); README PKM table and settings; verify with app tests and the menu/README key tests
- [x] 4.3 Exit with unsaved pages (`Dialog::UnsavedPages`: Save all / Don't Save / Cancel) after the map's question; verify with app tests

## 5. Editor window core

- [x] 5.1 `Editor` state and messages: load, active block as `text_editor`, splice-back and re-segmentation, cursor placement by byte offset, block navigation with arrows, Enter/Backspace/Tab rules wired through `key_binding`, undo/redo, Ctrl+S save, title with `*`, close with unsaved changes; verify with editor app tests (typing, Enter, undo, save BOM/CRLF, unsaved close)
- [x] 5.2 Markdown source highlighter (markers, emphasis fonts, code, links, fenced code with syntect) for the active block and source mode; verify highlight ranges in unit tests
- [x] 5.3 Block rendering: paragraphs, headings, lists with bullets/numbers/checkboxes, quotes, code blocks with syntax highlighting, thematic breaks, front matter, TOC, footnotes, link reference definitions, HTML blocks and placeholders, soft breaks; click-to-cursor mapping; verify mapping in unit tests and the look under Xvfb
- [x] 5.4 Scroll the document to keep the cursor visible and to jump to headings/matches; verify under Xvfb by typing at the bottom of a long page
- [x] 5.5 Editor menus (File, Edit, Paragraph, Format, Table, View) and the key map with US/Japanese layout characters; verify a test that every menu shortcut maps to its command

## 6. Rendering of math, diagrams and images

- [x] 6.1 Background SVG rendering with a cache for inline math, math blocks and Mermaid, errors shown below the source; verify with unit tests of the renderer wrapper and a page under Xvfb
- [x] 6.2 Image display with path resolution (relative, percent-decoded, `typora-root-url`, `file:`, absolute), missing and remote images as alt text with a link, scaling to the column; verify path resolution in unit tests and images under Xvfb

## 7. Editing features

- [ ] 7.1 Paragraph and format shortcuts, clear formatting, auto-pair (Edit menu toggle, remembered), selection helpers, task box clicks, Ctrl+click links (pages in the editor, anchors, files, URLs); verify with editor app tests
- [ ] 7.2 Multi-block selection (Shift+arrows at edges, Shift+click, second Ctrl+A) with copy/cut/delete/typing; verify with editor app tests
- [ ] 7.3 Find/replace bar (Ctrl+F, Ctrl+H, F3, Shift+F3, Esc, match case, Replace All as one change) with highlighted matches; verify with editor app tests
- [ ] 7.4 Source mode (Ctrl+/) keeping the cursor and history; outline sidebar (Ctrl+Shift+1); full screen (F11); zoom (Ctrl+Shift+=/-/0, remembered); light/dark theme (remembered); status bar counts; verify with editor app tests and under Xvfb

## 8. Tables in the editor

- [ ] 8.1 Grid editing of the active table: cell editor with `<br>`/`\|` conversion, Tab/Shift+Tab/Enter navigation with full selection, Ctrl+Enter, Ctrl+Shift+Backspace, Alt+arrows, Ctrl+L/Ctrl+E; verify with editor app tests
- [ ] 8.2 Table menu, right-click menu, toolbar (alignment, resize, delete table), cell range selection by drag and Shift+arrows, Delete on ranges; verify with editor app tests and under Xvfb

## 9. Clipboard and images

- [ ] 9.1 Clipboard through `arboard` behind an injectable trait: copy as HTML + Markdown, Copy as Markdown, Paste as Plain Text, Smart Paste with `htmd`, own copies pasted verbatim, tables as TSV/HTML; verify with editor app tests using a fake clipboard
- [ ] 9.2 Image insertion: drag and drop (several files), Format → Image dialog, pasted image data (PNG) and files, copying into the image folder with free names, settings and front matter options, path writing (`./`, escaping); verify with unit tests and editor app tests in a temporary directory; README image section

## 10. Export

- [ ] 10.1 HTML export (style sheet or `export.css`, inline SVG math/diagrams, image paths relative to the output) and PDF export through Edge/Chrome headless with `CREATE_NO_WINDOW`, `PDF export needs Microsoft Edge or Google Chrome.` when missing; verify HTML output in unit tests and a PDF exported under Linux with Chromium; README export section

## 11. Open pages and the PKM commands

- [ ] 11.1 Rename (F2, page and directory), Add Assets and Delete (F11) save affected dirty editors first, then retarget/reload/close them; reload or ask on external changes (focus and tick); verify with app tests for each scenario; README notes

## 12. Integration

- [ ] 12.1 README editor section and CLAUDE.md structure/capabilities updated; verify `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `openspec validate --all --strict` pass
- [ ] 12.2 Run the app under Xvfb: open a page from the map with F1, edit each element type, tables, images, find, source mode, dark theme, export; look at screenshots and fix what looks wrong

## Workflow follow-up

- Archive the change (`/opsx:archive`) once implemented, so the specs describe the editor.
- Bump the version in `Cargo.toml` to release.
