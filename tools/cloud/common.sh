#!/usr/bin/env bash
# Source from a platform adapter after changing to the repository root.
cloud_install_rust() {
  rustup toolchain install
}

cloud_prepare_dependencies() {
  cargo fetch --locked
  # curl uses the cloud proxy's supplied CA trust. Lindera verifies this cache
  # again itself. These parameters match lindera-ipadic 6.2.0 in Cargo.lock;
  # review them when upgrading the dictionary dependency (never weaken checks).
  local cache=.cache/lindera/6.2.0-fmt2
  local archive="$cache/mecab-ipadic-2.7.0-20250920.tar.gz"
  local checksum=a95c409f12f1023fce8ef91f991ef042
  mkdir -p "$cache"
  if ! printf '%s  %s\n' "$checksum" "$archive" | md5sum -c - >/dev/null 2>&1; then
    local download
    download=$(mktemp "$cache/download.XXXXXX")
    if ! curl -fL --max-time 90 https://lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz -o "$download"; then
      rm -f "$download"
      return 1
    fi
    if ! printf '%s  %s\n' "$checksum" "$download" | md5sum -c -; then
      rm -f "$download"
      return 1
    fi
    mv "$download" "$archive"
  fi
  cargo test --locked --no-run
}
