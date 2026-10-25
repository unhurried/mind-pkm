# Proposal

## Why

Pages open in whatever application Windows associates with `.md`, so the PKM workflow depends on a separately installed editor (Typora in practice) and the application knows nothing about the pages being edited. A built-in WYSIWYG Markdown editor, opened by default, makes the application self-contained and lets it keep open pages in step with the PKM commands that rename, extend or delete them.

The editor follows the requirements document "Markdown エディタ要求" (26/10/24), adjusted with the user where iced cannot provide something or where it would not fit the application (see Adjustments below).

## What Changes

- **Built-in editor, one window per page**: Open (F1), New Page (F9), opening a search result and clicking a node's link open `.md` files in an editor window of their own; opening a page that is already open brings its window to the front. A PKM menu toggle switches back to the default application, and a new PKM command, Open Externally, opens the selected pages there regardless.
- **Block-level live preview**: every block (paragraph, heading, list item, quote, code block, table, ...) is shown rendered, except the block holding the cursor, which shows its Markdown source - styled (bold in bold, markers dimmed, headings at heading size). Typing a syntax converts it right away (`# ` makes a heading, ` ``` ` + Enter a code block, a header row + Enter a table). Ctrl+/ switches to a source mode showing the whole file as text.
- **Plain Markdown**: the file is the source the user edits; nothing is re-serialized, so unknown syntax and HTML are kept byte for byte. The BOM and line endings of the file are kept.
- **Markdown elements**: CommonMark + GFM (tables, task lists, strikethrough, autolinks, footnotes), front matter, `[toc]`, math (`$...$`, `$$...$$`), sub/superscript, `==highlight==`, `:emoji:` shortcodes, Mermaid diagrams, images, and a subset of inline HTML.
- **Editing**: Enter / Shift+Enter, list continuation, Typora's shortcuts (headings, lists, quote, code, math, bold/italic/underline/code/strikethrough, link, image, table), auto-pairing (can be turned off), clear formatting, selection helpers (Ctrl+D/L/E), find and replace (Ctrl+F/H, F3/Shift+F3), undo/redo, word and character counts (also of the selection), multi-block selection.
- **Tables**: cell editing with Tab/Shift+Tab/Enter navigation, add/delete/move rows and columns, alignment, resize, delete, cell range selection with TSV/HTML copy and paste, through a Table menu, a right-click menu and a toolbar shown above the active table.
- **Images**: inserted by typing, drag and drop, a file dialog or pasting; copied into the page's `<name>.assets/` folder (settings and front matter can change the folder, `./` prefix and `%20` escaping) and written as a path relative to the page.
- **Clipboard**: copying puts HTML (rich text) and the Markdown on the clipboard; Copy as Markdown, Paste as Plain Text; pasting HTML from other applications converts it to Markdown (Smart Paste).
- **View**: outline sidebar (Ctrl+Shift+1), full screen (F11), zoom (Ctrl+Shift+= / - / 0), light and dark themes.
- **Export**: HTML (self-contained, math and diagrams as inline SVG, replaceable CSS) and PDF (the HTML printed by Microsoft Edge's headless mode).
- **Saving**: explicit Ctrl+S; closing a page window or the application with unsaved pages asks first; a page changed on disk is reloaded (or the user is asked when it has unsaved changes).
- **PKM commands and open pages**: Edit (F2) renaming, Add Assets and Delete (F11) save an open page first, then the editor follows the result (new path / reloaded text / window closed).
- **Multiple windows**: the application becomes a multi-window application; the map window keeps its behavior, and the arrow pointer applies to every window.

### Adjustments agreed with the user

1. Source is revealed per block, not per inline element; a selection across blocks is made of whole blocks (character-level selection across blocks in source mode).
2. Themes are built-in light/dark instead of replaceable CSS; CSS is replaceable for HTML/PDF export only.
3. `<iframe>`, `<video>` and other embedded content are shown as a placeholder with a link instead of being played; `<u>`, `<span style="color">`, `<br>`, `<sub>`, `<sup>`, `<mark>`, `<kbd>`, `<img>` are rendered.
4. Subscript and superscript are shown smaller, without raising or lowering the baseline.
5. Table rows and columns are moved with Alt+arrow keys and menu entries instead of drag and drop.
6. PDF export needs Microsoft Edge (or Chrome); without it only HTML export is offered.
7. Math and Mermaid are rendered by pure-Rust renderers; what they cannot render is reported below the block with the source kept.
8. Remote (http/https) images are not downloaded; their alt text and URL are shown.
9. Image insertion defaults to `<name>.assets/`, relative, no `./`, no escaping (the existing `[assets](Name.assets)` style, which F2 renaming rewrites); global options in `settings.txt`, per-page options in Typora's front matter keys.
10. A single newline inside a paragraph is shown as a line break, as Typora does.

## Capabilities

### New Capabilities
- `page-editor`: the editor windows - opening pages in them, the block-level live preview and source mode, typing and keyboard editing, undo, find and replace, selection, clipboard, counts, view options, saving and unsaved changes, and keeping open pages in step with the PKM commands.
- `markdown-rendering`: which Markdown (and HTML) the editor understands and how each element is shown, including math, Mermaid diagrams and images.
- `table-editing`: editing GFM tables cell by cell, their rows, columns, alignment and size, and cell ranges.
- `image-insertion`: inserting images into a page and copying them next to it.
- `page-export`: exporting a page to HTML and PDF.

### Modified Capabilities
- `document-management`: Open (F1) and New Page (F9) open pages in the built-in editor by default; new Open Externally command; the built-in editor toggle.
- `document-search`: opening a result that is a Markdown file uses the built-in editor.
- `mindmap-editing`: clicking a node's link marker opens a linked Markdown file in the built-in editor.
- `application-shell`: several windows (closing the map window ends the application and asks about unsaved pages too), new PKM menu entries, the arrow pointer in every window, new per-user settings.

## Impact

- New UI-free module `src/markdown/` (block segmentation, inline model, edits, tables, find, counts, image paths, HTML export, undo history) and UI module `src/app/editor/` (editor windows: state, view, keys, highlighter, tables, clipboard).
- `src/app/mod.rs` moves from `iced::application` to `iced::daemon` (multi-window): window ids in messages and subscriptions, per-window title, view and theme.
- `src/app/command.rs`, `keys.rs`, `view.rs` (PKM menu), `dialog.rs` (rename/add assets/delete with open pages, unsaved pages on exit), `search_panel.rs`, `settings.rs`.
- New dependencies: `pulldown-cmark` (parsing, HTML export), `latex-rust` (math), `mermaid-rs-renderer` (diagrams), `syntect` + `two-face` (code highlighting, pure-Rust regex), `arboard` (HTML/image/file clipboard), `emojis`, `htmd` (Smart Paste), `image` (PNG encoding of pasted images), `unicode-width`, `unicode-segmentation`; iced features `image-without-codecs` and `svg`.
- README (editor section, PKM table, settings), CLAUDE.md (structure, capabilities).
