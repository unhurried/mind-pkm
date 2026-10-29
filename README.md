# Mind PKM

Mind PKM (formerly Freeplane PKM) is a personal knowledge management (PKM) application built around a mind map. It started as a set of [Freeplane](https://freeplane.org/) add-on scripts ([unhurried/freeplane-pkm](https://github.com/unhurried/freeplane-pkm)); this is the same system rebuilt as a standalone Windows application in Rust with [iced](https://iced.rs/), so Freeplane (and Java) are no longer needed.

It reads and writes the same files as the add-on: your Freeplane `.mm` map and the document directory of Markdown pages, directories and attachments stay exactly as they are, and the map can still be opened in Freeplane at any time.

### Key Features

- **Mind Map Editing**: View and edit Freeplane `.mm` maps - nodes, folding, icons, links, undo/redo, cut/copy/paste
- **Document Management**: Manage links between mind map nodes and Markdown files/directories
- **Page Editor**: A built-in Markdown editor in the style of Typora - the page rendered block by block and edited in that form, with the Markdown of only the inline element at the cursor shown; tables, images, math, Mermaid diagrams, HTML/PDF export - see [Page Editor](#page-editor) below
- **Full-Text Document Search**: Indexed keyword search across your Markdown, PDF and Office document files, with Japanese morphological analysis - see [Full-Text Document Search](#full-text-document-search) below
- **Mind Map Search and Filtering**: Filter mind map nodes by conditions
- **Task Management**: Reflect tasks written in Markdown files into the mind map; ToDo items whose task has been removed from its page are dropped automatically; Next Steps items starting with a `yy/MM/dd` due date of today or earlier are collected under ToDo > Due Today / Past Due unless already in ToDo

## Installation

1. Download `mind-pkm.exe` from the [releases page](../../releases) and put it anywhere.
2. Run it, and open your map with File → Open. The last opened map is reopened on the next start, with the zoom level, scroll position and selected nodes it was last shown with (remembered per map in `%APPDATA%\mind-pkm\views.txt`), in a window of the size and position it last had (`window=` in `%APPDATA%\mind-pkm\settings.txt`; remove that line if the window ends up off-screen); a map path given on the command line (or opening a `.mm` file with the exe from Explorer) takes precedence.

The UI uses the Meiryo UI font; where it is missing, the system's fallback fonts are used.

There is nothing to install and no runtime to set up: the exe embeds everything, including the Japanese dictionary for the search index.

### Code Signing Certificate

`mind-pkm.exe` is signed with a self-signed certificate, `Mind PKM Code Signing` (SHA-1 thumbprint `8A2D49B3A38A3F6933F3367FA23663A694A5C597`, valid until 2036-10-11). Windows doesn't know that certificate, so until you install it the signature shows as untrusted. The certificate, `codesign.cer`, is attached to each release (it is also [`assets/codesign.cer`](assets/codesign.cer) in this repository).

To trust it for the current user only (no administrator rights needed), install it into the current user's **Trusted Root Certification Authorities** and **Trusted Publishers** stores. In PowerShell, in the folder holding `codesign.cer`:

```powershell
Import-Certificate -FilePath .\codesign.cer -CertStoreLocation Cert:\CurrentUser\Root
Import-Certificate -FilePath .\codesign.cer -CertStoreLocation Cert:\CurrentUser\TrustedPublisher
```

Windows asks for confirmation when a root certificate is added: check that the thumbprint shown matches the one above, and answer **Yes**. (`certutil -user -addstore Root codesign.cer` and `certutil -user -addstore TrustedPublisher codesign.cer` do the same from a command prompt.)

Or without a command line: double-click `codesign.cer` → **Install Certificate...** → Store Location **Current User** → **Place all certificates in the following store** → **Browse...** → **Trusted Root Certification Authorities** → **Finish**, and repeat the same with **Trusted Publishers**.

Check the result with `Get-AuthenticodeSignature .\mind-pkm.exe`: its `Status` is `Valid` once the certificate is trusted (the exe's Properties → Digital Signatures tab says "This digital signature is OK" as well). The certificate can't issue other certificates (it is not a CA), so trusting it trusts only what is signed with this certificate. To remove it again:

```powershell
Get-ChildItem Cert:\CurrentUser\Root, Cert:\CurrentUser\TrustedPublisher |
  Where-Object Thumbprint -eq 8A2D49B3A38A3F6933F3367FA23663A694A5C597 | Remove-Item
```

SmartScreen judges downloads by reputation rather than by the certificate, so it may still warn the first time you run a downloaded exe ("More info" → "Run anyway").

## The Map and the Document Directory

The conventions are those of the add-on, unchanged:

- The document directory is read from the map: `root > config > docDirPath > <path>` (an absolute path; a relative one is taken relative to the map file).
- **Page node**: a node whose text is `<name>` and which links `<docDir>/<name>.md`. **Directory node**: links the directory `<docDir>/<name>/`. A page's attachments live in `<docDir>/<name>.assets/`. Pages are UTF-8 with a BOM.
- `root > ToDo` holds the task list. Anything under a node named `archive` is hidden by the default filter. Dates are written `yy/MM/dd`.
- The map grows to the right only: every child of the root is shown on the right of it (a node Freeplane placed on the left included), and nodes added, pasted or moved under the root get `POSITION` "right", so Freeplane shows them on the right too.
- Links are written the way Freeplane writes them (relative to the map file), and anything in the map this application doesn't use itself - styles, attributes, rich text, hooks - is kept as it is when the map is saved.

File → New creates a map skeleton with `config > docDirPath` (edit its child to your document directory) and `ToDo`.

## PKM Commands

The PKM menu holds the commands of the former add-on, with the same function keys (and Ctrl shortcuts as aliases for some):

| Shortcut | Menu entry |
| --- | --- |
| F1 / Ctrl+O | Open |
| F2 | Edit |
| F3 | Fold One Level |
| F4 | Unfold One Level |
| F5 / Ctrl+Shift+F | Filter |
| F6 / Ctrl+F | Search |
| F7 | Toggle Checkmark |
| F8 | Add to ToDo |
| F9 / Ctrl+N | New Page |
| F10 / Ctrl+Shift+N | New Directory |
| F11 / Delete | Delete |
| Ctrl+F11 / Ctrl+Delete | Delete without Confirmation |
| F12 | Update Next Steps and Search Index |
| | Add Assets |
| | Find Unlinked |
| | Open Externally |
| | Use Built-in Editor |

- **Open**: opens the selected page nodes' `.md` files in the built-in editor (see [Page Editor](#page-editor)) and directory nodes' directories in Explorer.
- **Edit**: renames a page (its `.md` file, its `.assets` directory, the references inside the page and the ToDo items copied from it) or a directory along with the node; any other node just gets its text edited. Pages open in the editor are saved first and stay open under their new name.
- **Fold / Unfold One Level**: folds the deepest unfolded level / unfolds the shallowest folded level under the selected node.
- **Filter**: `a` shows all nodes, `t` the ToDo list, `d` the items due today or overdue, any other text the nodes containing it (case-insensitive); an empty command hides archived nodes (the default). Cancel keeps the current filter.
- **Toggle Checkmark**: toggles Freeplane's checkmark icon.
- **Add to ToDo**: copies the selected items (children of page nodes) to the top of `root > ToDo` as `<item> (<page>)`, linked to the page.
- **New Page**: creates `<docDir>/<text>.md` from the page template and the page's `.assets` directory, links the node to it and opens the page in the editor. The built-in template can be replaced by a `template.md` in `%APPDATA%\mind-pkm\` (`${page_assets_name}` and `${today}` are substituted). Settings left in `%APPDATA%\freeplane-pkm\` by versions under the former name are still read.
- **New Directory**: creates `<docDir>/<text>/`, links the node to it and opens it.
- **Delete**: deletes the selected nodes, after one confirmation listing them all, and moves the files of page and directory nodes to the Recycle Bin (where they can be restored from); any other node is deleted without touching files. A page or directory whose files can't be moved there keeps its node. It never touches the document directory itself or anything outside it. Open editors of deleted pages are saved first and close.
- **Delete without Confirmation**: Delete, but without the confirmation (hold Ctrl with F11 or Delete).
- **Update Next Steps and Search Index**: syncs up to 3 list items under each page's `### Next Steps` heading into the page node's children, drops the ToDo items whose task is gone from their page, fills ToDo > Due Today / Past Due, and updates the search index. This also runs automatically every few minutes while the map is being changed.
- **Add Assets**: creates the page's `.assets` directory and links it from the top of the page (an open editor of the page is saved first and shows the link afterwards).
- **Find Unlinked**: lists the pages, directories and orphaned `.assets` directories of the document directory that no node links, under `root > unlinked`.
- **Open Externally**: opens the selected pages (and directories) with the application Windows associates with them, whatever editor Open uses.
- **Use Built-in Editor**: checked by default; unchecked, Open, New Page, search results and link markers open pages with the default application (as versions before 0.12 did). The choice is remembered.

Page and directory names can't be empty, `.` or `..`, contain control characters or `\ / : * ? " < > |`, or be Windows device names (`CON`, `AUX`, `NUL`, `COM1`, ...); directory names can't end with a dot or a space either.

With several nodes selected, Open, Delete and Add to ToDo act on all of them (Delete asks once, listing every target, unless Ctrl is held), Toggle Checkmark, Fold/Unfold, New Page and New Directory run on each, and Edit and Add Assets ask for a single node to be selected.

## Editing the Map

| Shortcut | Action |
| --- | --- |
| Click / Ctrl+Click / Shift+Click | Select a node / toggle it in the selection / add it to the selection |
| Drag empty map space | Scroll (pan) the map |
| Drag a node onto another | Move the selected nodes: onto the middle of the target as its last child, onto its top / bottom edge before / after it (Esc cancels) |
| Double-click | Edit (as F2) |
| Arrow keys, Home | Move between nodes, go to the root |
| Tab / Insert | New child node |
| Enter | New sibling node |
| Ctrl+E | Edit node text |
| Delete / Ctrl+Delete | Delete the selected nodes and the files of pages and directories, with / without a confirmation (as F11) |
| Space | Fold / unfold |
| Ctrl+Up / Ctrl+Down | Move the node up / down among its siblings |
| Ctrl+X / Ctrl+C / Ctrl+V | Cut / copy / paste nodes (text from other applications is pasted one node per line) |
| Ctrl+K | Set or remove a node's link |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Esc | Center the selected node (or close the search panel / a dialog) |
| Ctrl+Plus / Ctrl+Minus / Ctrl+0, Ctrl+mouse wheel | Zoom in / zoom out / reset to 100% (View menu); the wheel zooms around the pointer |
| Ctrl+S / Ctrl+Shift+S | Save / save as |

Clicking a node's link marker opens the linked file, directory or URL, or jumps to the linked node; clicking the point where the lines to a node's children branch folds the node, and clicking the number next to a folded node unfolds it. The map is only written when you save; closing the window with unsaved changes asks first.

## Page Editor

Open (F1), New Page (F9), search results and link markers open a page in a window of its own, one per page (opening a page that is open brings its window to the front). The page is shown rendered, block by block - paragraphs, headings, lists and task lists, quotes, code with syntax highlighting, tables, images, math, Mermaid diagrams, footnotes, front matter, `[toc]` - and you edit it in that form, as in Typora: only the inline element at the cursor (bold text, a link, an image, a formula...) shows its Markdown, dimmed, until the cursor moves on; headings keep their size, list items their bullet or checkbox, quotes their bar, without `#`, `-` or `>`, also inside quotes and list items (a list or a heading in a quote, a quote or code in a list item are blocks of their own). A code block is edited as its code, in its box and highlighting, without the fences, with a field for its language at the bottom right (Down at the last line of a code block that ends the page starts a new paragraph after it; Backspace in an empty one removes it). Math blocks, diagrams, front matter and HTML blocks show their source while the cursor is in them.

Blocks are started by typing their Markdown at the start of a line: `#` shows as you typed it until the space after it makes the line a heading (`# `), and likewise `- `, `* `, `1. `, `- [ ] ` and `> ` make a list item, a task or a quote, which you then keep typing into rendered. ` ``` `, `$$`, `---`, a setext underline or `[toc]` stay text until Enter completes them (Enter after ` ``` ` adds the closing fence; after `---`, a horizontal rule and a new paragraph). Inline Markdown (`**bold**`, `[link](x.md)`, `$x^2$`...) is rendered as soon as it is complete, and shows its Markdown, dimmed around the styled text, while the cursor touches it. Click into a block to edit it; Ctrl+click follows a link (a page opens in the editor, `#anchor` jumps to its heading, anything else opens with its default application); clicking a task box ticks it.

The file stays plain Markdown and only the edited parts change: the UTF-8 BOM and the line endings (CRLF or LF) are kept. Ctrl+S saves (the title shows `*` while there are unsaved changes); closing the window or the application with unsaved changes asks first. A page changed on disk by another application is reloaded when the window is activated, or - with unsaved changes - you are asked which version to keep. Files other than `.md`, and every page when PKM → Use Built-in Editor is unchecked, open with their default application instead.

| Shortcut | Action |
| --- | --- |
| Enter / Shift+Enter | New paragraph (continuing a list or quote; an empty item ends the list, an empty paragraph leaves the quote or list item) / line break within the block |
| Tab / Shift+Tab | Indent / outdent a list item; next / previous cell in a table |
| Up / Down at a block's edge, Shift+Up / Shift+Down | Into the next block; select whole blocks |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl+X / Ctrl+C / Ctrl+V | Cut / copy (as rich text and Markdown) / paste (HTML from other applications becomes Markdown, a spreadsheet range a table, an image an inserted image) |
| Ctrl+Shift+C / Ctrl+Shift+V | Copy as Markdown / paste as plain text |
| Ctrl+A (twice: all blocks), Ctrl+D, Ctrl+L, Ctrl+E | Select the block's text, a word, a line (a table row), styled text (a table cell) |
| Ctrl+F / Ctrl+H, F3 / Shift+F3 | Find / replace, next / previous match |
| Ctrl+1 ... Ctrl+6, Ctrl+0, Ctrl+= / Ctrl+- | Heading level, paragraph, raise / lower the heading level |
| Ctrl+T, Ctrl+Shift+K, Ctrl+Shift+M, Ctrl+Shift+Q, Ctrl+Shift+[ / ] | Table, code block, math block, quote, ordered / bullet list |
| Ctrl+B / Ctrl+I / Ctrl+U, Ctrl+Shift+\`, Alt+Shift+5, Ctrl+K, Ctrl+\\ | Bold / italic / underline, code, strikethrough, link, clear formatting |
| Ctrl+Shift+I | Insert images |
| Ctrl+/ | Source mode (the whole page as Markdown) |
| Ctrl+Shift+1, Ctrl+Shift+2, F11, Ctrl+Shift+= / - / 0 | Outline, assets pane, full screen, zoom in / out / reset |
| Ctrl+S, Ctrl+W | Save, close |

The menus (File, Edit, Paragraph, Format, Table, View) hold every command with its shortcut, among them Export HTML / PDF, Highlight, Inline Math, Horizontal Rule, Auto-Pair Brackets and Dark Theme. On a Japanese keyboard the shortcuts with `=` and `` ` `` also work on the keys printed `;` and `@`. Brackets and quotes are closed as you type them (Edit → Auto-Pair Brackets turns that off); with text selected, `*`, `_`, `~` and `$` enclose it. The status bar counts words and characters of the page (or the selection), each CJK character counting as a word.

- **Tables**: while the cursor is in a table, it is edited as a grid of rendered cells (the cell being edited shows the Markdown at its cursor, like any block): Tab / Shift+Tab / Enter move between cells (adding a row at the end), Shift+Enter breaks a line in a cell (stored as `<br>`), Ctrl+Enter adds a row, Ctrl+Shift+Backspace deletes it, Alt+arrows move the row or column. Drag over cells, Shift+click or Shift+arrows select a range of cells, which Delete empties and Ctrl+C copies as tab-separated text and an HTML table. The toolbar above the table aligns columns, resizes the table and deletes it; the Table menu and the cells' right-click menu insert, delete and move rows and columns. A changed table is written with aligned pipes.
- **Images**: type `![alt](path)`, drop image files on the window, choose Format → Image..., or paste a screenshot or copied image files. Images are copied into the page's `<name>.assets/` folder (a taken name gets a number, pasted image data is saved as `image-<date and time>.png`) and referenced relatively, like `![](My Page.assets/photo 1.png)`, so that renaming the page (F2) keeps them working. Remote images are not downloaded: their alt text is shown with a link.
- **Assets pane**: View → Assets (Ctrl+Shift+2) shows the page's `<name>.assets/` folder as a tree on the right of the window. Double-click opens a file (a `.md` file in the editor), right-click renames it (rewriting the page's links to it) or deletes it to the Recycle Bin, and dragging it onto the page inserts a link to it where it is dropped (an image as `![](...)`).
- **Math and diagrams**: `$...$` and `$$` blocks (LaTeX) and ` ```mermaid ` blocks are drawn below their source while you edit them, with any error shown there.
- **Export**: File → Export HTML... writes a self-contained HTML file (math and diagrams as SVG, images referenced relative to it). File → Export PDF... prints that page to a PDF with Microsoft Edge (or Google Chrome) in the background. The style sheet can be replaced by an `export.css` in `%APPDATA%\mind-pkm\`.
- **HTML in pages**: `<u>`, `<mark>`, `<sub>`, `<sup>`, `<kbd>`, `<br>`, `<img>` and `<span style="color: ...">` are rendered; other tags (`<iframe>`, `<video>`, ...) show as a placeholder with a link.

The editor remembers its window size, zoom, theme and the auto-pairing switch. These lines in `%APPDATA%\mind-pkm\settings.txt` change how images are inserted:

| Setting | Effect |
| --- | --- |
| `image_copy=0` | Reference images where they are instead of copying them |
| `image_folder=<folder>` | Copy images into `<folder>` (relative to the page; `${filename}` is the page's name) instead of `${filename}.assets` |
| `image_dot_slash=1` | Write paths as `./My Page.assets/...` |
| `image_escape=1` | Escape spaces and other special characters in paths (`%20`) |

A page's front matter can set its own image folder with `typora-copy-images-to: <folder>`, and `typora-root-url: <folder>` resolves image paths starting with `/`.

## Full-Text Document Search

Press `F6` (or `Ctrl+F`) to open the search panel under the map: type keywords and press Enter, and matching files show up in a result list with a short content snippet, the matched keywords highlighted. Double-click a result (or select it and press Enter) to open it: a page in the editor, anything else with its default application.

- **Formats**: Markdown/plain text, PDF, and current-format Office documents (`.docx`, `.xlsx`, `.pptx`). Any other file type is still searchable by file name.
- **Matching**: multiple space-separated keywords (a full-width space works too) are combined with AND, matched case-insensitively against both file content and file name. Japanese is analyzed morphologically (with the IPADIC dictionary), so `京都` doesn't match `東京都` and `読む` matches `読んだ`.
- **Sorting**: the order box (`Last modified` / `Relevance`) orders results by last-modified time, newest first (the default), or by relevance (file-name matches rank above content-only matches).
- **Filtering**: the scope box (`Pages` / `Attachments` / `Both`) limits results to pages (the default), attachments, or both. A page is a `.md` file outside any `<page>.assets/` directory; every other file counts as an attachment.
- **Jumping to the map node**: a result's node is the node that links its file, its containing directory, or - for a file under a page's `.assets/` directory - its page. Select the result and press `Ctrl+Enter` or use `Select Node` to select and center that node in the map.
- **Index**: kept at `<docDir>/.search-index/tantivy/` (with `tantivy.meta` recording each indexed file's last-modified time, to skip unchanged files on the next update). It is refreshed whenever the search panel opens, every few minutes while the map is being changed, and on F12. The Lucene index the Freeplane add-on kept in the same directory is left alone, so both can be used side by side.

## Development

### Requirements

- Rust, the version in `rust-toolchain.toml` (edition 2024) - see [rustup.rs](https://rustup.rs/); `mise install` installs it too
- Building downloads the IPADIC dictionary once, into `.cache/lindera/` (it is embedded into the binary).

### Build and Test

```bash
cargo test                       # all tests
cargo clippy --all-targets       # lints
cargo fmt --check                # formatting
cargo build --release            # target/release/mind-pkm(.exe)
```

The application is developed for Windows but builds and runs on Linux too, which is handy for development; `cargo build --release --target x86_64-pc-windows-gnu` cross-compiles the Windows exe from Linux (with the MinGW toolchain installed).

### Dev Container

The [devcontainer-claude-code](https://github.com/unhurried/devcontainer-claude-code) dev container has everything else this repository needs. Clone this repository under its `repos/` and, with `PROXY_MODE=open` (crates.io, npm and the IPADIC dictionary are outside its allowlist), set it up once:

```bash
mise trust && mise install            # Rust (rust-toolchain.toml), Node.js, the OpenSpec CLI
cargo fetch --locked                  # the crates
cargo test --locked --no-run          # also downloads the IPADIC dictionary
```

Then switch back to the allowlist: building, testing and `openspec validate` need no network until the dependencies change (`Cargo.lock`, the OpenSpec CLI in `mise.toml`); a new toolchain in `rust-toolchain.toml` only needs `mise install`. To look at the UI, run the app under Xvfb and drive it with `xdotool` (that repository's README, GUI apps).

### Source Layout

- `src/xml.rs`, `src/mindmap.rs` - reading/writing `.mm` files without losing anything
- `src/paths.rs` - link URI <-> file path conversion, path comparison
- `src/pkm/` - the PKM conventions and commands (`commands.rs`), the Next Steps refresh (`next_steps.rs`), the filter (`filter.rs`)
- `src/search/` - the full-text index (tantivy + lindera), text extraction from PDF/Office files
- `src/markdown/` - Markdown for the page editor without UI: blocks and inline content (pulldown-cmark), the editing rules, tables, find, word counts, HTML and PDF export
- `src/app/` - the iced UI: state and messages (`mod.rs`), the open map and its undo history (`doc.rs`), commands (`command.rs`), keyboard (`keys.rs`), dialogs (`dialog.rs`), file handling (`file.rs`), scrolling (`viewport.rs`), drag and drop of nodes (`drag.rs`), rendering (`view.rs`), map layout (`layout.rs`), the search panel (`search_panel.rs`), the page editor's windows (`editor/`)
- `src/lib.rs` - the library holding all of the above (`src/main.rs` only starts the application); `tests/` - property tests against it

### Specifications

The application's behavior is specified with [OpenSpec](https://github.com/Fission-AI/OpenSpec) in `openspec/specs/` - one `spec.md` per capability (`mindmap-file`, `mindmap-editing`, `application-shell`, `document-management`, `task-management`, `map-filter`, `document-search`, `page-editor`, `markdown-rendering`, `table-editing`, `image-insertion`, `page-export`), as requirements with scenarios. Behavior changes go through an OpenSpec change (`openspec/changes/`; in Claude Code: `/opsx:propose`, `/opsx:apply`, `/opsx:archive`).

```bash
mise install                          # Node.js, the OpenSpec CLI and Rust (mise.toml)
openspec list --specs                 # the capabilities
openspec show document-search --type spec
openspec validate --all --strict      # also run by CI
```

### Release via GitHub Actions

The `Build` workflow (`.github/workflows/build.yml`) runs the checks and tests on Windows on every push/PR and uploads the exe as a build artifact. A push to `main` (e.g. merging a pull request) additionally creates a GitHub release with the exe and `codesign.cer` attached: it tags the commit `v<version>`, with the version from `Cargo.toml`, and generates the release notes from the merged pull requests. If that tag already exists, no release is made, so to release, bump `version` in `Cargo.toml` (and `Cargo.lock`, e.g. with `cargo check`) in the pull request.

The workflow signs the exe (with `signtool`, SHA-256, timestamped) using two repository secrets: `CODESIGN_PFX`, the base64-encoded PKCS#12 file with the certificate and its private key, and `CODESIGN_PASSWORD`, its password. Without them the exe is left unsigned (a warning), except for a release, which then fails. `tools/make_codesign_cert.sh <out-dir>` creates a new certificate: it writes the PFX, its base64 form and the password to `<out-dir>` (keep these out of the repository) and the public certificate to `assets/codesign.cer`; update the thumbprint and expiry date above when you replace it.

## License

MIT - see [LICENSE](LICENSE).
