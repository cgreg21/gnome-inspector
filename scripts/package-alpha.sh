#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)"
if [[ -z "$VERSION" ]]; then
    echo "Could not read the package version from Cargo.toml." >&2
    exit 1
fi

ARCH="$(uname -m)"
PACKAGE="gnome-inspector-$VERSION-linux-$ARCH"
DIST_DIR="${GNOME_INSPECTOR_DIST_DIR:-$ROOT_DIR/dist}"
BUILD_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
if [[ "$BUILD_DIR" != /* ]]; then
    BUILD_DIR="$ROOT_DIR/$BUILD_DIR"
fi

(
    cd "$ROOT_DIR"
    cargo build --release
)

BINARY="$BUILD_DIR/release/gnome-inspector"
if [[ ! -x "$BINARY" ]]; then
    echo "Release binary not found: $BINARY" >&2
    exit 1
fi

STAGE_DIR="$(mktemp -d)"
trap 'rm -rf -- "$STAGE_DIR"' EXIT
PACKAGE_DIR="$STAGE_DIR/$PACKAGE"
mkdir -p "$PACKAGE_DIR/bin" "$PACKAGE_DIR/extension"
install -m 755 "$BINARY" "$PACKAGE_DIR/bin/gnome-inspector"
cp -a "$ROOT_DIR/extension/." "$PACKAGE_DIR/extension/"
install -m 755 "$ROOT_DIR/scripts/install-bundle.sh" "$PACKAGE_DIR/install.sh"
install -m 755 "$ROOT_DIR/scripts/uninstall-bundle.sh" "$PACKAGE_DIR/uninstall.sh"
install -m 755 "$ROOT_DIR/scripts/check-runtime.sh" "$PACKAGE_DIR/check-runtime.sh"
install -m 644 "$ROOT_DIR/README.md" "$PACKAGE_DIR/README.md"
install -m 644 "$ROOT_DIR/CHANGELOG.md" "$PACKAGE_DIR/CHANGELOG.md"
install -m 644 "$ROOT_DIR/LICENSE" "$PACKAGE_DIR/LICENSE"

mkdir -p "$DIST_DIR"
tar -C "$STAGE_DIR" -czf "$DIST_DIR/$PACKAGE.tar.gz" "$PACKAGE"
echo "Created $DIST_DIR/$PACKAGE.tar.gz"
