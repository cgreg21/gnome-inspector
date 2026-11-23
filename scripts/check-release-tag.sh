#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TAG="${1:-}"
VERSION="$(awk -F '"' '/^version = / { print $2; exit }' "$ROOT_DIR/Cargo.toml")"

if [[ -z "$TAG" || -z "$VERSION" ]]; then
    echo "Usage: $0 v<package-version>" >&2
    exit 1
fi

if [[ "$TAG" != "v$VERSION" ]]; then
    echo "Tag $TAG does not match package version v$VERSION." >&2
    exit 1
fi

echo "Release tag matches package version: $TAG"
