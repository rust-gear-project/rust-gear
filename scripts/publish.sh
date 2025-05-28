#!/bin/bash
set -euo pipefail

TARGET_DIR="${1:-$(pwd)/npm}"

NORMALIZED_DIR=$(realpath "$TARGET_DIR")

if [ ! -d "$NORMALIZED_DIR" ]; then
  echo "Error: Target directory not found: $NORMALIZED_DIR" >&2
  exit 1
fi

cd "$NORMALIZED_DIR"
echo "📦 Publishing from: $(pwd)"

for platform in */; do
  if [ -f "${platform}package.json" ]; then
    echo "➡️ Publishing: $platform"
    (cd "$platform" && npm publish --access public)
  else
    echo "⚠️ Skipped (no package.json): $platform"
  fi
done