#!/usr/bin/env sh
# Prints the `[workspace.package]` version from Cargo.toml (e.g. `0.8.0`) - the single source of
# truth for the released version. Shared by scripts/bump-docs-version.sh (which stamps the pinned
# Action refs in the docs) and by action.yml (which defaults the binary version to the one bundled
# at the pinned action ref), so both read one definition rather than each parsing Cargo.toml.
set -eu

cd "$(dirname "$0")/.."

version=$(awk '
  /^\[workspace\.package\]/ { in_ws = 1; next }
  /^\[/ { in_ws = 0 }
  in_ws && /^version *= *"/ {
    match($0, /"[^"]+"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
  }
' Cargo.toml)

if [ -z "$version" ]; then
  echo "error: could not read [workspace.package] version from Cargo.toml" >&2
  exit 1
fi

printf '%s\n' "$version"
