#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(awk -F '"' '/^version = / { print $2; exit }' "$ROOT_DIR/Cargo.toml")"

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
