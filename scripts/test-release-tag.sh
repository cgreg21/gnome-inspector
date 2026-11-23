#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\\([^"]*\\)"/\\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)"

bash "$ROOT_DIR/scripts/check-release-tag.sh" "v$VERSION"
if bash "$ROOT_DIR/scripts/check-release-tag.sh" "v$VERSION-wrong"; then
    echo "A mismatched release tag should have been rejected." >&2
    exit 1
fi
if bash "$ROOT_DIR/scripts/check-release-tag.sh"; then
    echo "A missing release tag should have been rejected." >&2
    exit 1
fi

echo "Release tag validation tests passed."
