# Design

## Context

See proposal.md for why. The application is a single-window `iced::application` (iced 0.14): `App` holds one map (`Doc`), dialogs, the search panel; `update` dispatches by topic and relayouts the map; keys come from `keyboard::listen()` and are mapped by `command_for_key`. Pages are opened with `opener::open`. Files are read and written synchronously by the PKM commands; anything slow runs through `blocking(...)`.

What iced offers for an editor (checked against the 0.14 sources):

- `text_editor`: multi-line editing with selection, word motions, clipboard, IME preedit/commit, wrapping, an auto-growing height, a custom `key_binding` hook (a key can become a custom message) and a line-based `Highlighter` that may set color and font per range - but one font size per editor, no images or widgets inside, and no undo. `Content` exposes the cursor as line + byte column.
- `rich_text` with spans (size, font, color, link, underline, strikethrough, background); `image` and `svg` widgets; `table`; `row().wrap()` (flowing elements); widget operations (focus, scroll, custom `Operation`s for bounds); `iced::daemon` for several windows; `window::Event::FileDropped`; `window::set_mode` (full screen).

Probes run during planning: `latex-rust` (pure Rust, STIX Two Math) typesets fractions, roots, sums, integrals, matrices and `cases` to SVG in under 1 ms but fails on CJK inside `\text{}`; `mermaid-rs-renderer` renders flowcharts and sequence diagrams to SVG in 1-25 ms, with Japanese labels through the system fonts, and reports syntax errors with a position; `arboard` reads and writes HTML, images and file lists on Windows.

## Goals / Non-Goals

**Goals:**
- The page file is the single source of truth: the editor edits its text and never regenerates it from a parse tree, so everything it does not understand survives unchanged.
- Reuse iced's `text_editor` for all character-level editing (IME, selection, wrapping) instead of writing an editing widget.
- Keep the Markdown logic UI-free and unit-testable, like `pkm/`; the app layer only maps it to widgets and messages.

**Non-Goals:**
- Inline-level source reveal, character selection across blocks outside source mode, CSS themes, playing embedded media, downloading remote images (see the proposal's adjustments).
- Spell checking, autosave, version history, collaborative editing, editing non-UTF-8 files.
- Making the map window dark.

## Decisions

### Windows

`run` switches to `iced::daemon`; `boot` opens the map window itself (same geometry settings as today). `App` gains `main_window: window::Id` and `editors: BTreeMap<window::Id, Editor>`. `view`, `title` and `theme` take the window id and dispatch. Keyboard input moves from `keyboard::listen()` to `event::listen_with`, which carries the window id: `Message::Key(id, event)` goes to the map handling for the map window and to the editor otherwise. `resize_events`, `close_requests`, moves and file drops are routed the same way. All existing map messages stay window-less; editor messages are `Message::Editor(window::Id, EditorMsg)`.

Separate windows rather than a pane keep the editor's F3/F11/Ctrl+F/Ctrl+K etc. from colliding with the map's function keys and shortcuts - each window has its own key map - and match how pages opened in an external editor before.

Closing: an editor's close request runs its own unsaved check (a dialog in that window). The map window's close request (and Exit) runs the existing map check and then, when editors are dirty, a new `Dialog::UnsavedPages` in the map window (Save / Don't Save / Cancel) before `iced::exit()`.

### Document model (`src/markdown/`)

The editor holds `text: String` (LF line endings, BOM stripped; the original line ending style and BOM are remembered and restored on save - always with a BOM for pages as `pkm::write_page` does) and re-derives everything else from it after each change:

- `segment(text) -> Vec<Block>`: pulldown-cmark (0.13, with tables, footnotes, strikethrough, task lists, math, YAML metadata, sub/superscript) over the whole text with offsets; top-level events become blocks with byte ranges; lists are flattened so that each item is its own block (its range stops where its first nested list starts; content after a nested list becomes a continuation block); non-blank text no event covers (link reference definitions) becomes a dimmed `Other` block. Every non-blank byte belongs to exactly one block (property-tested). Parsing a page takes well under a millisecond, so re-segmenting on each keystroke is fine.
- Lenient link destinations: before parsing, a destination with spaces and no angle brackets (`](My Page.assets)`) is wrapped in `<...>`; the inserted bytes are recorded and offsets mapped back, so block ranges stay true to the original text.
- `inline(block_text) -> Vec<Inline>`: the rendering model of a block's inline content - styled text runs (bold, italic, strike, underline, code, highlight, sub/sup, color, link), each carrying the source byte range it came from, plus math, images, footnote references, breaks and HTML placeholders. `==x==` and `:shortcode:` are recognized in text events (not inside code); a small inline-HTML recognizer handles the supported tags.
- Edits are pure functions `(text, selection) -> (text, selection)` (Enter/Backspace rules, format toggles, heading levels, list/quote toggles, clear formatting, auto-pair, task toggles), table operations work on a parsed `Table` model and re-serialize the table, find/replace returns match ranges, counts are computed from the inline model. All of these are unit-tested without iced.
- Undo: a history of `(text, selection)` snapshots of the whole text (pages are small), consecutive typing coalesced into one step (same block, contiguous, within 1 s, broken at whitespace), at most 500 steps.

Alternatives: an AST that is serialized back (Typora's approach) would normalize unedited Markdown and lose unknown syntax - ruled out by the plain-Markdown requirement. A rope is unnecessary at page sizes.

### The active block

The block containing the cursor is the active block; it is shown as a `text_editor` whose `Content` holds exactly that block's text. After every action on the `Content`, its text is spliced back into `text` (replacing the block's old range), the document is re-segmented and the block containing the cursor is determined again. If its range no longer equals the `Content`'s (the user typed a blank line, an unclosed fence, a lazy continuation...), the `Content` is rebuilt from the new block with the cursor mapped through byte offsets. Clicking a rendered block, moving across a block edge, find results and undo all go through the same "put the cursor at byte offset N" operation.

The active editor gets the block's presentation: its font size is the heading size for headings; code, math and front matter use the code font on a code background; quotes and list items keep their indentation and bar/bullet column beside the editor. A `Highlighter` implementation (line-based, with fence/math/front-matter state across lines) styles the source: markers dimmed, emphasis in bold/italic fonts, code in the code font, links colored, and fenced code highlighted with syntect for its language. Source mode is the same `text_editor` over the whole text with the same highlighter.

Keys: the editor's `key_binding` hook turns every editor shortcut, Enter/Shift+Enter/Tab/Backspace/Delete rules, arrows at block edges, auto-pair characters, Ctrl+Z/Y and clipboard keys into `EditorMsg`s; everything else falls through to iced's default binding. Up/Down at a block edge is detected by applying the motion and checking whether the cursor moved (iced's editor leaves it in place on the first/last visual line), so wrapped lines need no measuring. Shortcuts with layout-dependent characters (`=`, `+`, `;`, `[`, `` ` ``, `\`, `/`) accept the characters both US and Japanese layouts produce, like the map's zoom keys.

Clicks on rendered text: the block's spans are laid out with iced's `Paragraph` at the block's width (as `view.rs` already measures node texts), `hit_test` gives the rendered character, and the run's source range maps it to a byte offset. Non-text blocks (images, diagrams, code) map a click to their start or the nearest line.

Scrolling: the document is one `scrollable` column of blocks. The text editor does not scroll the outer scrollable to the cursor, so after cursor-moving actions a custom widget `Operation` reads the active block's bounds and the scrollable's viewport, the caret's line offset is measured with a `Paragraph` of the text before the cursor, and the scrollable is scrolled by the difference when the caret is outside the viewport.

### Rendering

Each block becomes widgets: a `rich_text` of its runs (headings sized, links as span links that emit `EditorMsg::LinkClicked`, Ctrl deciding between following and editing), lists as a bullet/number/checkbox column plus content, quotes with a left bar, code blocks highlighted with syntect (two-face syntaxes, `fancy-regex`, so no C dependency), tables with iced's `table`, images with `image`/`svg` widgets scaled to the column width. Paragraphs that mix text with inline math or inline images are split at line-break opportunities into a wrapping `row` so SVGs flow with the words; plain paragraphs keep one `rich_text`. Soft line breaks are shown as breaks.

Math and Mermaid are rendered to SVG off the UI thread (`blocking`) and cached by source text (`HashMap<String, Result<svg::Handle, String>>`); a block shows its source dimmed until its SVG arrives. The math SVG is scaled to the body font size. Emoji come from the `emojis` crate's GitHub shortcodes.

The view is rebuilt on every message like the map's; widgets of blocks that did not change are cheap to rebuild. If long pages turn out slow, blocks can be wrapped in iced's `lazy` keyed by their text without changing the design.

### Tables

When the active block is a table, it is shown as a grid of cells instead of source: the cursor's cell is a `text_editor` holding the cell's text with `<br>` shown as line breaks and `\|` as `|` (converted back on every change), other cells are rendered `rich_text` that activate on click. The `Table` model (header, alignments, rows) is parsed from the block, edited by table operations and re-serialized with padded columns, replacing the block's text as one undoable change. A toolbar row above the active table holds alignment buttons, rows × columns inputs with Resize, and Delete Table. The right-click menu is an overlay at the pointer (mouse area `on_right_press` + the last `on_move` position), built like the map's menu drop-down. Cell ranges are tracked as (anchor cell, head cell); dragging uses `mouse_area` enter events while the button is down.

### Clipboard

iced's clipboard is text-only, so the editor uses `arboard` for everything (HTML + text on copy, HTML/image/file list/text on paste). Clipboard reads and writes run through `blocking`. Copy renders the selected Markdown fragment to HTML with the export renderer (without the page wrapper). To paste our own copies back verbatim, the editor remembers the last text it put on the clipboard (as the map does) and prefers it over converting the HTML. Smart Paste converts HTML with `htmd`. Tests inject a fake clipboard.

### Images

`markdown::images` computes, for a page and an image file, the target folder (`image_folder` setting / `typora-copy-images-to` front matter / `<name>.assets`), a free file name, and the reference text (relative with `/`, optional `./` and `%XX` escaping); copying runs in `blocking`. Dropped files arrive as `window::Event::FileDropped` per file; drops within 100 ms are inserted together. Pasted image data is encoded as PNG with the `image` crate (PNG codec only; iced's image feature without codecs plus the codecs the editor shows).

### Export

`markdown::html` renders the whole page with pulldown-cmark's HTML writer over the same event stream the editor uses (lenient links, highlight, emoji, sub/superscript), replacing math and Mermaid with their SVGs and rewriting local image `src` relative to the output file. The style sheet is built in (`assets/export.css`) unless `export.css` exists in the settings directory. PDF: the HTML is written to a temporary file with absolute image URIs and printed with `msedge --headless --disable-gpu --no-pdf-header-footer --print-to-pdf=<out> <file URI>` (Chrome as a fallback; found in the standard install locations, and on Linux on the `PATH` for development), with `CREATE_NO_WINDOW`, in `blocking`.

### Saving, reloading and PKM commands

Save writes synchronously in `update` like the map (pages are small): text with the original line endings, through `pkm::write_page`. Each editor remembers the file's modification time after loading/saving; on window focus and on the 20 s tick it compares, and reloads or asks (spec: Files changed on disk).

Rename (F2), Add Assets and Delete (F11) are run by the app layer, which knows the editors: before running them it saves the dirty editors whose path is the affected page (or under the renamed directory) - a failed save aborts the command with the error - and afterwards updates their paths and reloads them, or closes them for deleted files. `pkm::commands` stays unaware of editors.

Opening: `App::open_page(path)` is the single entry used by F1, New Page, search results, node link markers and links inside pages; it checks the Use Built-in Editor setting and `.md` extension, focuses an existing window (path compared with `paths::key`), or reads the file and opens a window.

### Settings

New `settings.txt` keys: `page_editor=external` (absent = built-in), `editor_window=w,h`, `editor_zoom=`, `editor_theme=dark`, `auto_pair=0`, `image_copy=0`, `image_folder=`, `image_dot_slash=1`, `image_escape=1`; plus the optional `export.css` file. All are read through `settings.rs` (none in tests).

### Pointer and fonts

Every window's view is wrapped in `arrow_cursor`, so the editor shows the arrow over text as well (spec: Arrow pointer). Body text uses the application font (Meiryo UI); code uses Consolas on Windows (`Font::MONOSPACE` elsewhere).

### Testing

- `src/markdown/` unit tests for segmentation, lenient links, inline runs, every edit rule, tables (parse/serialize, operations), find/replace, counts, image paths, HTML export; property tests: segmentation covers all non-blank text exactly once, table parse∘serialize is stable, an edit inside the active block never changes text outside it.
- `src/app/editor/tests.rs` drives editor windows through messages without a window (iced `Content` works headless, as the map tests' text measuring does): opening from F1/search/link, typing, Enter rules, undo, save (BOM, CRLF), unsaved close, exit with unsaved pages, rename/add assets/delete with open pages, reload on external change, find/replace, table keys, image drop with a temporary directory and a fake clipboard.
- The key binding table is checked against the editor menus as `keys.rs` does for the map.
- Run under Xvfb, drive with xdotool, and look at screenshots of each element type, the table editor, dark theme, source mode.

## Risks / Trade-offs

- [Rebuilding `Content` when block boundaries change could disturb an IME composition] → the preedit lives in the widget state and the text only changes on commit; rebuilds happen only after edits, never during composition.
- [Block granularity shows a whole long paragraph's markers while it is edited] → accepted adjustment 1; styled source keeps it readable.
- [The view rebuilds every block on each keystroke] → cheap for typical pages; `lazy` per block if profiling shows a problem.
- [Mermaid/math renderer coverage is narrower than mermaid.js/KaTeX] → errors are shown below the source; nothing is lost from the file.
- [PDF export depends on Edge's command-line flags] → flags present since Chromium 112; failures are reported with the browser's exit status.
- [Multi-window conversion touches every window event] → existing app tests keep covering the map window; new tests cover routing by window id.
- [Binary size grows (syntaxes, math font)] → a few MB next to the embedded dictionary; acceptable.

## Migration Plan

No data migration: pages, maps and settings keep their formats. Users who prefer their external editor uncheck PKM → Use Built-in Editor once. Rollback is installing the previous version.
