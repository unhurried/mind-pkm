# Mind PKM

Mind PKM (formerly Freeplane PKM) is a personal knowledge management (PKM) application built around a mind map. It started as a set of [Freeplane](https://freeplane.org/) add-on scripts ([unhurried/freeplane-pkm](https://github.com/unhurried/freeplane-pkm)); this is the same system rebuilt as a standalone Windows application in Rust with [iced](https://iced.rs/), so Freeplane (and Java) are no longer needed.

It reads and writes the same files as the add-on: your Freeplane `.mm` map and the document directory of Markdown pages, directories and attachments stay exactly as they are, and the map can still be opened in Freeplane at any time.

### Key Features

- **Mind Map Editing**: View and edit Freeplane `.mm` maps - nodes, folding, icons, links, undo/redo, cut/copy/paste
- **Document Management**: Manage links between mind map nodes and Markdown files/directories
- **Full-Text Document Search**: Indexed keyword search across your Markdown, PDF and Office document files, with Japanese morphological analysis - see [Full-Text Document Search](#full-text-document-search) below
- **Mind Map Search and Filtering**: Filter mind map nodes by conditions
- **Task Management**: Reflect tasks written in Markdown files into the mind map; ToDo items whose task has been removed from its page are dropped automatically; Next Steps items starting with a `yy/MM/dd` due date of today or earlier are collected under ToDo > Due Today / Past Due unless already in ToDo

## Installation

1. Download `mind-pkm-<version>-windows-x64.zip` from the [releases page](../../releases) and extract `mind-pkm.exe` anywhere.
2. Run it, and open your map with File → Open (`Ctrl+O`). The last opened map is reopened on the next start; a map path given on the command line (or opening a `.mm` file with the exe from Explorer) takes precedence.

The UI uses the Meiryo UI font; where it is missing, the system's fallback fonts are used.

There is nothing to install and no runtime to set up: the exe embeds everything, including the Japanese dictionary for the search index.

### Code Signing Certificate

`mind-pkm.exe` is signed with a self-signed certificate, `Mind PKM Code Signing` (SHA-1 thumbprint `8A2D49B3A38A3F6933F3367FA23663A694A5C597`, valid until 2036-10-11). Windows doesn't know that certificate, so until you install it the signature shows as untrusted. The certificate, `codesign.cer`, is included in the zip and attached to each release (it is also [`assets/codesign.cer`](assets/codesign.cer) in this repository).

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

The PKM menu holds the commands of the former add-on, with the same keyboard shortcuts:

| Shortcut | Menu entry |
| --- | --- |
| F1 | Open |
| F2 | Edit |
| F3 | Fold One Level |
| F4 | Unfold One Level |
| F5 | Filter |
| F6 | Search |
| F7 | Toggle Checkmark |
| F8 | Add to ToDo |
| F9 | New Page |
| F10 | New Directory |
| F11 | Delete |
| F12 | Update Next Steps and Search Index |
| | Add Assets |
| | Find Unlinked |

- **Open**: opens the selected page nodes' `.md` files / directory nodes' directories with the default application.
- **Edit**: renames a page (its `.md` file, its `.assets` directory, the references inside the page and the ToDo items copied from it) or a directory along with the node; any other node just gets its text edited.
- **Fold / Unfold One Level**: folds the deepest unfolded level / unfolds the shallowest folded level under the selected node.
- **Filter**: `a` shows all nodes, `t` the ToDo list, `d` the items due today or overdue, any other text the nodes containing it (case-insensitive); an empty command hides archived nodes (the default). Cancel keeps the current filter.
- **Toggle Checkmark**: toggles Freeplane's checkmark icon.
- **Add to ToDo**: copies the selected items (children of page nodes) to the top of `root > ToDo` as `<item> (<page>)`, linked to the page.
- **New Page**: creates `<docDir>/<text>.md` from the page template and the page's `.assets` directory, links the node to it and opens the page. The built-in template can be replaced by a `template.md` in `%APPDATA%\mind-pkm\` (`${page_assets_name}` and `${today}` are substituted). Settings left in `%APPDATA%\freeplane-pkm\` by versions under the former name are still read.
- **New Directory**: creates `<docDir>/<text>/`, links the node to it and opens it.
- **Delete**: deletes the selected page and directory nodes, after one confirmation listing them all, and moves their files to the Recycle Bin (where they can be restored from). A page or directory whose files can't be moved there keeps its node. It never touches the document directory itself or anything outside it.
- **Update Next Steps and Search Index**: syncs up to 3 list items under each page's `### Next Steps` heading into the page node's children, drops the ToDo items whose task is gone from their page, fills ToDo > Due Today / Past Due, and updates the search index. This also runs automatically every few minutes while the map is being changed.
- **Add Assets**: creates the page's `.assets` directory and links it from the top of the page.
- **Find Unlinked**: lists the pages, directories and orphaned `.assets` directories of the document directory that no node links, under `root > unlinked`.

Page and directory names can't be empty, `.` or `..`, contain control characters or `\ / : * ? " < > |`, or be Windows device names (`CON`, `AUX`, `NUL`, `COM1`, ...); directory names can't end with a dot or a space either.

With several nodes selected, Open, Delete and Add to ToDo act on all of them (Delete asks once, listing every target), Toggle Checkmark, Fold/Unfold, New Page and New Directory run on each, and Edit and Add Assets ask for a single node to be selected.

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
| Delete | Delete the selected nodes (the files are kept; see F11) |
| Space | Fold / unfold |
| Ctrl+Up / Ctrl+Down | Move the node up / down among its siblings |
| Ctrl+X / Ctrl+C / Ctrl+V | Cut / copy / paste nodes (text from other applications is pasted one node per line) |
| Ctrl+K | Set or remove a node's link |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Esc | Center the selected node (or close the search panel / a dialog) |
| Ctrl+Plus / Ctrl+Minus / Ctrl+0, Ctrl+mouse wheel | Zoom in / zoom out / reset to 100% (View menu); the wheel zooms around the pointer |
| Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S | New / open / save / save as |

Clicking a node's link marker opens the linked file, directory or URL, or jumps to the linked node; clicking the point where the lines to a node's children branch folds the node, and clicking the number next to a folded node unfolds it. The map is only written when you save; closing the window with unsaved changes asks first.

## Full-Text Document Search

Press `F6` to open the search panel under the map: type keywords and press Enter, and matching files show up in a result list with a short content snippet, the matched keywords highlighted. Double-click a result (or select it and press Enter) to open it with its default application.

- **Formats**: Markdown/plain text, PDF, and current-format Office documents (`.docx`, `.xlsx`, `.pptx`). Any other file type is still searchable by file name.
- **Matching**: multiple space-separated keywords (a full-width space works too) are combined with AND, matched case-insensitively against both file content and file name. Japanese is analyzed morphologically (with the IPADIC dictionary), so `京都` doesn't match `東京都` and `読む` matches `読んだ`.
- **Sorting**: the order box (`Last modified` / `Relevance`) orders results by last-modified time, newest first (the default), or by relevance (file-name matches rank above content-only matches).
- **Filtering**: the scope box (`Pages` / `Attachments` / `Both`) limits results to pages (the default), attachments, or both. A page is a `.md` file outside any `<page>.assets/` directory; every other file counts as an attachment.
- **Jumping to the map node**: a result's node is the node that links its file, its containing directory, or - for a file under a page's `.assets/` directory - its page. Select the result and press `Ctrl+Enter` or use `Select Node` to select and center that node in the map.
- **Index**: kept at `<docDir>/.search-index/tantivy/` (with `tantivy.meta` recording each indexed file's last-modified time, to skip unchanged files on the next update). It is refreshed whenever the search panel opens, every few minutes while the map is being changed, and on F12. The Lucene index the Freeplane add-on kept in the same directory is left alone, so both can be used side by side.

## Development

### Requirements

- Rust (stable, edition 2024) - see [rustup.rs](https://rustup.rs/)
- Building downloads the IPADIC dictionary once (it is embedded into the binary).

### Build and Test

```bash
cargo test                       # all tests
cargo clippy --all-targets       # lints
cargo fmt --check                # formatting
cargo build --release            # target/release/mind-pkm(.exe)
```

The application is developed for Windows but builds and runs on Linux too, which is handy for development; `cargo build --release --target x86_64-pc-windows-gnu` cross-compiles the Windows exe from Linux (with the MinGW toolchain installed).

### Source Layout

- `src/xml.rs`, `src/mindmap.rs` - reading/writing `.mm` files without losing anything
- `src/paths.rs` - link URI <-> file path conversion, path comparison
- `src/pkm/` - the PKM conventions and commands (`commands.rs`), the Next Steps refresh (`next_steps.rs`), the filter (`filter.rs`)
- `src/search/` - the full-text index (tantivy + lindera), text extraction from PDF/Office files
- `src/app/` - the iced UI: state and messages (`mod.rs`), the open map and its undo history (`doc.rs`), commands (`command.rs`), keyboard (`keys.rs`), dialogs (`dialog.rs`), file handling (`file.rs`), scrolling (`viewport.rs`), drag and drop of nodes (`drag.rs`), rendering (`view.rs`), map layout (`layout.rs`), the search panel (`search_panel.rs`)
- `src/lib.rs` - the library holding all of the above (`src/main.rs` only starts the application); `tests/` - property tests against it

### Specifications

The application's behavior is specified with [OpenSpec](https://github.com/Fission-AI/OpenSpec) in `openspec/specs/` - one `spec.md` per capability (`mindmap-file`, `mindmap-editing`, `application-shell`, `document-management`, `task-management`, `map-filter`, `document-search`), as requirements with scenarios. Behavior changes go through an OpenSpec change (`openspec/changes/`; in Claude Code: `/opsx:propose`, `/opsx:apply`, `/opsx:archive`).

```bash
mise install                          # Node.js and the OpenSpec CLI (mise.toml)
openspec list --specs                 # the capabilities
openspec show document-search --type spec
openspec validate --all --strict      # also run by CI
```

### Release via GitHub Actions

The `Build` workflow (`.github/workflows/build.yml`) runs the checks and tests on Windows on every push/PR and uploads the exe as a build artifact. A push to `main` (e.g. merging a pull request) additionally creates a GitHub release with the zipped exe attached: it tags the commit `v<version>`, with the version from `Cargo.toml`, and generates the release notes from the merged pull requests. If that tag already exists, no release is made, so to release, bump `version` in `Cargo.toml` (and `Cargo.lock`, e.g. with `cargo check`) in the pull request.

The workflow signs the exe (with `signtool`, SHA-256, timestamped) using two repository secrets: `CODESIGN_PFX`, the base64-encoded PKCS#12 file with the certificate and its private key, and `CODESIGN_PASSWORD`, its password. Without them the exe is left unsigned (a warning), except for a release, which then fails. `tools/make_codesign_cert.sh <out-dir>` creates a new certificate: it writes the PFX, its base64 form and the password to `<out-dir>` (keep these out of the repository) and the public certificate to `assets/codesign.cer`; update the thumbprint and expiry date above when you replace it.

## License

MIT - see [LICENSE](LICENSE).
