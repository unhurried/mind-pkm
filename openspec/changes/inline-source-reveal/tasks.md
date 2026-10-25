# Tasks

## 1. Display model (`src/markdown/`)

- [ ] 1.1 Inline elements with their source ranges from the inline builder (parser tags, code spans, math, footnote references, HTML tag pairs, `==`/`~`/`^` pairs, escapes, entities, emoji, `<br>`); verify with unit tests per element
- [ ] 1.2 `live::Display`: units (one-to-one runs, atomic units, objects) with styles, hidden bytes owned by the innermost element, revealed elements from the cursor, shown/source offset mapping; verify with unit tests and a property test that every shown offset maps into the block and back
- [ ] 1.3 Edits on shown text as source edits (deleting with markup kept or removed, inserting at the cursor, word and grapheme motions, select word / all); verify each scenario of page-editor's "Editing rendered text" in unit tests, plus a property test that deleting and retyping a shown range restores the shown text

## 2. Layout and widget (`src/app/editor/`)

- [ ] 2.1 Pure line breaking over measured clusters and objects (break opportunities, kinsoku, long runs, hanging spaces, baselines) with hit testing, caret and selection geometry, vertical and Home/End targets; verify with unit tests on synthetic measurements
- [ ] 2.2 `LiveText` widget: measuring style runs with a per-run paragraph cache, drawing (backgrounds, clipped fragments, underline/strikethrough, images, SVG, caret blink), mouse (click, double/triple click, Shift+click, drag, Ctrl+click links), keys through the binding function, IME requests and commits, `Focusable`, caret rectangle through a custom operation

## 3. Editor integration

- [ ] 3.1 `Active` with a live region for paragraphs, headings, list items, quotes and footnote definitions (bullet/number/checkbox, quote bar and heading size drawn around it); `Page::perform` translating text editor actions to live edits; editing rules called with source positions; undo; find marks; reveal scrolling through the caret operation; update the editor app tests from source-showing to rendered editing
- [ ] 3.2 Table cells as live regions (cell Markdown with `<br>` and `\|`, Tab/Enter/arrows/ranges as before); verify the table tests and a cell with bold text
- [ ] 3.3 Keep `text_editor` for code-like blocks and source mode; verify code block, math block and source mode tests still pass

## 4. Integration

- [ ] 4.1 README (Page Editor section) and CLAUDE.md (structure); `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (also for the Windows target), `cargo test`, `openspec validate --all --strict`
- [ ] 4.2 Run the app under Xvfb: type, delete and select in formatted paragraphs, headings, lists, quotes, table cells; images and math in a paragraph being edited; Up/Down/Home/End; look at screenshots and fix what looks wrong
