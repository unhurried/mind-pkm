# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Mind PKM (formerly Freeplane PKM): a standalone Windows application (Rust + iced) for a personal knowledge management (PKM) system around a mind map. It replaces the Freeplane add-on [unhurried/freeplane-pkm](https://github.com/unhurried/freeplane-pkm) and must stay compatible with it: the same `.mm` files (Freeplane can still open them) and the same document directory layout.

## Commands

```bash
cargo test                                 # all tests
cargo test search::                        # one module
cargo clippy --all-targets -- -D warnings  # CI fails on any warning
cargo fmt --check
cargo build --release                      # target/release/mind-pkm(.exe)
cargo build --release --target x86_64-pc-windows-gnu   # Windows exe from Linux (needs mingw-w64)
```

The first build downloads the IPADIC dictionary (lindera's `embed-ipadic`), which is embedded in the binary. A push to `main` releases the `Cargo.toml` version as tag `v<version>` unless that tag exists (`.github/workflows/build.yml`, Windows runner) - bump the version to release.

The Rust toolchain is pinned in `rust-toolchain.toml` (CI reads it too); bump it on purpose, fixing new lints in the same change. In Claude Code cloud sessions `.claude/hooks/session-start.sh` installs that toolchain (rustup), Node.js and the OpenSpec CLI via [mise](https://mise.jdx.dev) (`mise.toml` pins the versions; CI uses the same file), Xvfb, xdotool and ImageMagick, and prebuilds the tests.

## Structure

- `src/lib.rs` holds every module; `src/main.rs` only starts the app. `tests/` holds integration and property tests (proptest) against the library.
- `src/xml.rs` - minimal XML DOM. Writes like Freeplane's own writer: no XML declaration, every non-ASCII UTF-16 unit as `&#x..;` (so characters outside the BMP become surrogate-pair references, which the reader recombines), newlines in attributes as `&#xa;`.
- `src/mindmap.rs` - the `.mm` model. Nodes live in an arena (`NodeId` = index, never reused - not even after an undo: `Doc` pads a restored map with `reserve_ids`, so replace `Doc::map` only through `Doc`); nodes and the ID index sit behind `Arc`s, so a copy (one per undo step) shares everything it doesn't change - mutate nodes only through `node_mut`; each node keeps all its XML attributes and non-node child elements (`Item::Xml`) in order, so anything the app doesn't understand round-trips unchanged. `MindMap::parse(..).to_xml()` must reproduce a Freeplane-written file byte for byte (see the round-trip test) - keep it that way.
- `src/paths.rs` - `LINK` URI <-> path (relative to the map file, percent-encoded like `java.net.URI`; `file:/C:/...` for absolute ones) and path comparison (`key`: normalized, case-folded on Windows, like `java.io.File#equals`).
- `src/pkm/` - the add-on's `Utils.groovy` and menu scripts, UI-free: an `Err(String)` is the message shown to the user, and commands that ask first are split into a check and an execution (`delete_targets` / `delete_question` / `delete_execute`). Error messages are the add-on's, verbatim.
- `src/pkm/next_steps.rs` - `snapshot` (UI thread) → `compute` (background: reads pages) → `apply` (UI thread; skips nodes deleted/renamed meanwhile).
- `src/search/` - tantivy index under `<docDir>/.search-index/tantivy/` + `tantivy.meta` (schema version + mtimes for incremental updates); the add-on's Lucene index (`lucene/`, `files.meta`) is deliberately left alone. Bump `INDEX_SCHEMA_VERSION` whenever the analyzer or fields change incompatibly; `analyzer.rs` tests pin the tokenization.
- `src/app/` - iced (0.14) UI. `mod.rs`: `App`, `Message`, `update` (dispatches each message to the module of its topic); `doc.rs`: `Doc` (an open map: selection, filter, layout, undo) and the edits that only touch the map; `command.rs`: `Command` and `run_command`; `keys.rs`: keyboard handling, key bindings (`command_for_key`); `dialog.rs`: dialogs and their outcome; `file.rs`: new/open/save, unsaved-changes guard; `viewport.rs`: relayout, scrolling, panning, zooming (`ctrl_wheel.rs` catches Ctrl+wheel over the map); `drag.rs`: moving nodes by drag and drop (drop position, `Doc::move_nodes`); `view.rs`: rendering, menus (`MENUS`), node measuring; `layout.rs`: pure mind map layout; `search_panel.rs`: F6 panel; `tests.rs`: the app driven by its messages (keys, commands, dialog answers, background results) without a window - add a test there for a UI flow. The layout is recomputed in `update` after every message (`relayout`), never in `view`; text sizes are measured with the same paragraph engine the text widget uses and cached per text. Zooming lays the map out at the zoomed size (font, paddings, gaps all scaled), so layout coordinates are always screen pixels; the text size cache holds the current zoom only.

- `assets/codesign.cer` - the public half of the self-signed code signing certificate (`tools/make_codesign_cert.sh`); the workflow signs the exe with the `CODESIGN_PFX` / `CODESIGN_PASSWORD` secrets. The private key never goes into the repository. README documents the thumbprint.
- `assets/icon.*` - the application icon, drawn by `tools/make_icon.py` (Pillow; rerun it after changing the drawing). `build.rs` embeds `icon.ico` into the Windows exe (winresource; `windres` from mingw-w64 when cross-compiling), and `icon-64.rgba` is the window icon.

## Conventions (not discoverable from the code alone)

- Document directory: `root > config > docDirPath > <path>`. Page node: text links `<docDir>/<text>.md`; directory node: `<docDir>/<text>/`; attachments in `<text>.assets/`. Pages are UTF-8 with a BOM (`pkm::read_page`/`write_page`). Dates are `yy/MM/dd`.
- Up to 3 items under a page's `### Next Steps` are synced into the node's children; ToDo items (`<item> (<page>)` linking the page) whose text is gone from *all* of the page's Next Steps are deleted; Due Today / Past Due are filled from all items. The refresh runs on F12, and every 5 minutes after the map changed.
- The map is laid out to the right only (`layout.rs` ignores `POSITION`); children added, pasted or moved under the root get `MindMap::set_right`. An existing node's `POSITION` is never rewritten unless it is moved.
- Every map mutation goes through `Doc::checkpoint()` first (undo), or through `Doc::try_change` when the change may fail (recorded only if it succeeds). Recording a change also marks the map as edited, which is what schedules the periodic refresh; the refresh applies its own changes through `Doc::apply_refresh`, which doesn't. Undo entries carry a serial number (`checkpoint` returns it); take back a specific change by its serial (`Doc::cancel_new_node`), never by popping the latest entry - a background refresh may have been recorded since.
- File I/O and indexing run via `blocking(...)` (tokio blocking pool), never in `update`/`view` - except the small, synchronous file operations of the PKM commands.
- Pull requests (title and description) are written in English.
- F1-F12 must keep the add-on's assignments (README table). Adding a command means: `Command` variant, `MENUS` entry, `command_for_key` if it has a shortcut, and the README; tests in `keys.rs` check that the menu shortcuts, the key bindings and the README's PKM table agree.

## Guardrails (never)

- Never remove, move or overwrite anything outside the document directory, nor the document directory itself. Paths built from node text must pass `paths::is_strictly_inside` (a node is a page/directory node only then), and deleting goes through `commands::delete_execute`, which checks again and takes the remover as a parameter: the app moves files to the Recycle Bin, tests remove them inside a temporary directory - never the real trash. `delete_never_leaves_the_document_directory` must keep passing; extend it rather than weaken it.
- Never change the `.mm` reader/writer without the byte-for-byte round-trip test passing, and never reword a ported command's message.
- Never skip, ignore or loosen a test, lint or `openspec validate` to get to green; fix the cause.

## Specifications (OpenSpec)

The behavior contract lives in `openspec/specs/<capability>/spec.md` (OpenSpec, `spec-driven` schema; `openspec/config.yaml` holds the project context). Capabilities: `mindmap-file`, `mindmap-editing`, `application-shell`, `document-management`, `task-management`, `map-filter`, `document-search`.

- A change of behavior starts as an OpenSpec change (`/opsx:propose`, or `openspec/changes/<name>/` with proposal, delta specs, design, tasks), is implemented with `/opsx:apply`, and is merged into the specs with `/opsx:archive`. Pure refactors and tooling changes need no spec change.
- Keep the specs about observable behavior - no module or function names.
- `openspec validate --specs --strict` must pass (CI runs it). The CLI is installed by `mise install` (`mise.toml`).

## Definition of done

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass, and `openspec/specs/` describes the new behavior. For UI changes, run the app (it works under Xvfb on Linux; `xdotool` can drive it) and look at it.
