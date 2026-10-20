#!/bin/bash
# Prepares a Claude Code cloud session so that the Definition of done in
# CLAUDE.md can be met: the pinned Rust toolchain (rust-toolchain.toml), the
# OpenSpec CLI at the version CI uses, the tools to run and drive the app under
# Xvfb, and the crates and a test build (the container is cached afterwards).
# The Rust toolchain comes from rustup, Node.js and the OpenSpec CLI from mise
# (mise.toml). Idempotent; does nothing outside cloud sessions.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-$(dirname "$0")/../..}"

# The toolchain named in rust-toolchain.toml, with clippy and rustfmt.
rustup toolchain install

# Node.js and the OpenSpec CLI at the versions in mise.toml (the build workflow
# uses the same file). The Rust toolchain stays with rustup.
export PATH="$HOME/.local/bin:$PATH"
if ! command -v mise >/dev/null 2>&1; then
  curl -fsSL https://mise.run | sh
fi
mise trust --quiet mise.toml
mise install --yes node "npm:@fission-ai/openspec"
if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  {
    echo 'export PATH="$PATH:$HOME/.local/bin:$HOME/.local/share/mise/shims"'
    echo 'export OPENSPEC_TELEMETRY=0'
  } >> "$CLAUDE_ENV_FILE"
fi

# Running the app under Xvfb and driving it (see CLAUDE.md).
packages=(xvfb xdotool imagemagick libxkbcommon-x11-0)
missing=()
for p in "${packages[@]}"; do
  dpkg -s "$p" >/dev/null 2>&1 || missing+=("$p")
done
if [ ${#missing[@]} -gt 0 ]; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get install -y -q "${missing[@]}" || { apt-get update -q && apt-get install -y -q "${missing[@]}"; }
fi

# Crates (including the IPADIC dictionary) and a test build, so the first
# `cargo test` of the session doesn't start from scratch.
cargo fetch --locked
cargo test --locked --no-run
