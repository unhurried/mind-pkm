# Cloud development environments

The Claude Code hook and Codex adapter share `common.sh`: the pinned Rust
installation, locked crate fetch, verified IPADIC archive cache, and test
prebuild. Tool versions come from `rust-toolchain.toml` and `mise.toml`.
The dictionary archive/hash/cache format matches Lindera 6.2.0; review these
parameters when updating Lindera. TLS and archive verification stay enabled.

Platform-specific setup stays in the adapters:

- `.claude/hooks/session-start.sh` keeps Claude's cloud-only guard,
  `CLAUDE_ENV_FILE` integration, mise and system package installation.
- `codex-install.sh` uses writable tool directories, the exact Node version in
  `mise.toml`, npm and a local extraction of signed Debian
  packages for Xvfb/xdotool. This adapter targets the current Debian trixie,
  x86-64 Codex image. If the image's Node differs, the adapter downloads the
  pinned runtime from nodejs.org and verifies its SHA-256 checksum before
  extracting it. The GUI package versions are pinned; update them from
  signed Debian metadata if the mirror rotates them out.

## Codex

From the existing checkout (each cloud task is isolated; no extra worktree is
needed), set the environment's install script to:

```bash
cd /workspace/mind-pkm
bash tools/cloud/codex-install.sh
```

The scripts must be committed and available on the selected repository ref
before switching the cloud configuration to this invocation. Setup installs and
prebuilds; it does not run the full tests. Run these when validating changes:

```bash
. /workspace/tooling/mind-env.sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
openspec validate --all --strict
```

Use `MIND_PKM_TOOLING_DIR` to override `/workspace/tooling` (an absolute writable
path). To activate tools in later shells, source its `mind-env.sh`. Dependencies
and caches survive snapshots; processes do not.

For GUI work:

```bash
bash tools/cloud/codex-start.sh /absolute/path/to/document.md
```

The launcher uses an existing `DISPLAY`, or starts a private Xvfb display,
checks it with `xdpyinfo`, prints the display number, and stops its own Xvfb
when the app exits. It uses `ICED_BACKEND=tiny-skia` because the default backend
showed a blank window in this cloud image. Runtime/cache/config files stay under
`/workspace/tooling/gui`. Use a disposable document for smoke tests; inspect the
rendered window and verify edits saved to disk.

## Settings outside Git

Outbound access and secret bindings are managed in the cloud environment's
settings. The scripts do not change them or publish an environment. Required
hosts include `sh.rustup.rs`, `static.rust-lang.org`, `index.crates.io`,
`static.crates.io`, `nodejs.org`, `registry.npmjs.org`, `lindera.dev`, and
`deb.debian.org`.
Claude's mise bootstrap additionally uses `mise.run` and its release download
hosts. Git access uses the platform's existing authentication; GitHub PR
operations require `api.github.com`. No application secrets are needed.
Never commit secret values. Review/save settings and publish prepared snapshots
through the cloud UI.
