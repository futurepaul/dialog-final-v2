#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version> (e.g., 0.1.1)" >&2
  exit 1
fi

# Accept either "0.1.1" or "VERSION=0.1.1"
RAW_ARG="$1"
if [[ "$RAW_ARG" == VERSION=* ]]; then
  VERSION="${RAW_ARG#VERSION=}"
else
  VERSION="$RAW_ARG"
fi
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"

cd "$ROOT_DIR"

# Ensure clean git state
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Git working tree not clean. Commit or stash changes first." >&2
  exit 1
fi

echo "Bumping versions to $VERSION..."

# macOS-compatible sed in-place helper
sed_inplace() {
  local file="$1" pattern="$2" replacement="$3"
  # shellcheck disable=SC2001
  local repl
  repl=$(echo "$replacement" | sed -e 's/[\//!]/\\&/g')
  sed -i '' -E "s/$pattern/$repl/" "$file"
}

# 1) Bump library package version (dialog_final_v2_lib)
LIB_TOML="dialog_lib/Cargo.toml"
if [[ -f "$LIB_TOML" ]]; then
  sed_inplace "$LIB_TOML" '^(version\s*=\s*")([^"]+)(")$' "\\1$VERSION\\3"
fi

# 2) Bump CLI package version (dialog_final_v2_cli)
CLI_TOML="dialog_cli/Cargo.toml"
if [[ -f "$CLI_TOML" ]]; then
  sed_inplace "$CLI_TOML" '^(version\s*=\s*")([^"]+)(")$' "\\1$VERSION\\3"
  # Also update dependency on the lib with new version
  sed_inplace "$CLI_TOML" '^(dialog_lib\s*=\s*\{[^}]*version\s*=\s*")([^"]+)("[^}]*\})$' "\\1$VERSION\\3"
fi

echo "Running cargo check..."
cargo check >/dev/null

echo "Committing release bump..."
git add "$LIB_TOML" "$CLI_TOML"
git commit -m "chore(release): v$VERSION"

echo "Tagging v$VERSION..."
git tag "v$VERSION"

cat <<EOF

Release prepared.

Next steps:
  - Push commits and tag:
      git push origin HEAD && git push origin v$VERSION
  - Publish crates (order matters):
      cargo publish -p dialog_final_v2_lib
      # Wait until the lib is available (30-60s)
      cargo publish -p dialog_final_v2_cli

Or run: just publish

EOF
