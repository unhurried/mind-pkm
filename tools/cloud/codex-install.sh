#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
repo_root=$PWD
tooling=${MIND_PKM_TOOLING_DIR:-/workspace/tooling}
mkdir -p "$tooling"
. tools/cloud/common.sh
export CARGO_HOME="$tooling/cargo" RUSTUP_HOME="$tooling/rustup"
export PATH="$tooling/cargo/bin:$PATH"
export CARGO_BUILD_JOBS=5 OPENSPEC_TELEMETRY=0 npm_config_cache="$tooling/npm-cache"
if ! command -v rustup >/dev/null; then
  curl -fsS https://sh.rustup.rs -o /tmp/mind-rustup.sh
  sh /tmp/mind-rustup.sh -y --no-modify-path --default-toolchain none --profile minimal --component clippy,rustfmt
fi
cloud_install_rust
node_major=$(sed -n 's/^node = "\([0-9]*\)"/\1/p' mise.toml)
node --version | grep -E "^v${node_major}\."
openspec_version=$(sed -n 's/^"npm:@fission-ai\/openspec" = "\([^"]*\)"/\1/p' mise.toml)
test -n "$node_major" && test -n "$openspec_version"
npm install --prefix "$tooling/openspec" --no-audit --no-fund "@fission-ai/openspec@$openspec_version"
if [ ! -x "$tooling/x11/root/usr/bin/Xvfb" ]; then
  mkdir -p "$tooling/apt/lists/partial" "$tooling/apt/cache/archives/partial" "$tooling/x11"
  printf '%s\n' 'deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://deb.debian.org/debian trixie main' > "$tooling/apt/sources.list"
  aptopts=(-o "Dir::Etc::sourcelist=$tooling/apt/sources.list" -o Dir::Etc::sourceparts=- -o "Dir::State::lists=$tooling/apt/lists" -o "Dir::Cache=$tooling/apt/cache")
  /usr/bin/apt-get "${aptopts[@]}" update
  cd "$tooling/x11"
  # This version was validated against this environment; refresh from signed
  # Debian metadata if the release rotates out of the official mirror.
  /usr/bin/apt-get "${aptopts[@]}" download xvfb=2:21.1.16-1.3+deb13u3 xserver-common=2:21.1.16-1.3+deb13u3 xdotool=1:3.20160805.1-5.1 libxdo3=1:3.20160805.1-5.1
  for package in *.deb; do dpkg-deb -x "$package" root; done
  cd "$repo_root"
fi
# Generate activation without embedding the caller's environment or secrets.
{
  printf 'export CARGO_HOME=%q\n' "$CARGO_HOME"
  printf 'export RUSTUP_HOME=%q\n' "$RUSTUP_HOME"
  printf 'export MIND_PKM_TOOLING_DIR=%q\n' "$tooling"
  cat <<'ENV'
export CARGO_BUILD_JOBS=5
export OPENSPEC_TELEMETRY=0
export npm_config_cache="$MIND_PKM_TOOLING_DIR/npm-cache"
export PATH="$MIND_PKM_TOOLING_DIR/cargo/bin:$MIND_PKM_TOOLING_DIR/openspec/node_modules/.bin:$MIND_PKM_TOOLING_DIR/x11/root/usr/bin:$PATH"
export LD_LIBRARY_PATH="$MIND_PKM_TOOLING_DIR/x11/root/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
ENV
} > "$tooling/mind-env.sh"
. "$tooling/mind-env.sh"
cloud_prepare_dependencies
cargo fmt --check
openspec validate --all --strict
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
