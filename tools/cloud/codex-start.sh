#!/usr/bin/env bash
# Start a native GUI using the retained Codex tools, with software rendering.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
tooling=${MIND_PKM_TOOLING_DIR:-/workspace/tooling}
. "$tooling/mind-env.sh"
mkdir -p "$tooling/gui/runtime" "$tooling/gui/cache" "$tooling/gui/config"
chmod 700 "$tooling/gui/runtime"
export XDG_RUNTIME_DIR="$tooling/gui/runtime"
export XDG_CACHE_HOME="$tooling/gui/cache"
export XDG_CONFIG_HOME="$tooling/gui/config"
export ICED_BACKEND=tiny-skia
if [ -z "${DISPLAY:-}" ]; then
  display_file=$(mktemp)
  Xvfb -displayfd 3 -screen 0 1280x900x24 -nolisten tcp 3>"$display_file" >"$tooling/gui/xvfb.log" 2>&1 &
  xvfb_pid=$!
  trap 'kill "$xvfb_pid" 2>/dev/null || true; rm -f "$display_file"' EXIT
  for ((attempt=0; attempt<100; attempt++)); do
    if [ -s "$display_file" ]; then break; fi
    if ! kill -0 "$xvfb_pid" 2>/dev/null; then
      cat "$tooling/gui/xvfb.log" >&2
      exit 1
    fi
    sleep 0.1
  done
  test -s "$display_file"
  export DISPLAY=":$(cat "$display_file")"
fi
xdpyinfo >/dev/null
printf 'Mind PKM display: %s\n' "$DISPLAY" >&2
# Keep this shell alive so EXIT cleans up only the Xvfb process it started.
target/debug/mind-pkm "$@"
