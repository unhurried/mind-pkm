#!/bin/bash
# Prepares a Claude Code cloud session so that the Definition of done in
# CLAUDE.md can be met: the pinned Rust toolchain (rust-toolchain.toml), the
# OpenSpec CLI at the version CI uses, the tools to run and drive the app under
# Xvfb, and the crates and a test build (the container is cached afterwards).
# The Rust toolchain comes from rustup, Node.js and the OpenSpec CLI from mise
# (mise.toml). Idempotent; does nothing outside cloud sessions. Progress goes to
# stderr (stdout of a SessionStart hook is added to the session's context).
set -euo pipefail

step() { echo "session-start: $*" >&2; }

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-$(dirname "$0")/../..}"

. tools/cloud/common.sh

# The toolchain named in rust-toolchain.toml, with clippy and rustfmt.
step "installing the Rust toolchain"
cloud_install_rust

# Node.js and the OpenSpec CLI at the versions in mise.toml (the build workflow
# uses the same file). The Rust toolchain stays with rustup.
step "installing Node.js and the OpenSpec CLI (mise)"
export PATH="$HOME/.local/bin:$PATH"
if ! command -v mise >/dev/null 2>&1; then
  curl -fsSL https://mise.run | sh
fi
mise trust --quiet mise.toml
mise install --yes node "npm:@fission-ai/openspec"
# The hook runs at every session start, resume, clear and compact: add each line
# to the session's environment file only once.
if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  for line in \
    'export PATH="$PATH:$HOME/.local/bin:$HOME/.local/share/mise/shims"' \
    'export OPENSPEC_TELEMETRY=0'; do
    grep -qxF -- "$line" "$CLAUDE_ENV_FILE" 2>/dev/null || echo "$line" >> "$CLAUDE_ENV_FILE"
  done
fi

# Running the app under Xvfb and driving it (see CLAUDE.md).
packages=(xvfb xdotool imagemagick libxkbcommon-x11-0)
missing=()
for p in "${packages[@]}"; do
  dpkg -s "$p" >/dev/null 2>&1 || missing+=("$p")
done
if [ ${#missing[@]} -gt 0 ]; then
  step "installing apt packages: ${missing[*]}"
  export DEBIAN_FRONTEND=noninteractive
  apt_install=(apt-get install -y -q --no-install-recommends "${missing[@]}")
  "${apt_install[@]}" || { apt-get update -q && "${apt_install[@]}"; }
fi

# Crates (including the IPADIC dictionary) and a test build, so the first
# `cargo test` of the session doesn't start from scratch.
step "fetching crates and building the tests"
cloud_prepare_dependencies
