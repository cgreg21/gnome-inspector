#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${GNOME_INSPECTOR_BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_PATH="$BIN_DIR/gnome-inspector"
BUILD_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
if [[ "$BUILD_DIR" != /* ]]; then
    BUILD_DIR="$ROOT_DIR/$BUILD_DIR"
fi
DESKTOP_DIR="$DATA_DIR/applications"
DESKTOP_FILE="$DESKTOP_DIR/io.github.cgreg21.GnomeInspector.desktop"

if ! command -v cargo >/dev/null 2>&1; then
    echo "Cargo is required. Install Rust stable before continuing." >&2
    exit 1
fi

if ! command -v gnome-extensions >/dev/null 2>&1; then
    echo "gnome-extensions is required to install the GNOME Shell agent." >&2
    exit 1
fi

echo "Building GNOME Inspector in release mode..."
(
    cd "$ROOT_DIR"
    cargo build --release
)

if [[ ! -x "$BUILD_DIR/release/gnome-inspector" ]]; then
    echo "The release binary was not produced." >&2
    exit 1
fi

install -d "$BIN_DIR" "$DESKTOP_DIR"
install -m 755 "$BUILD_DIR/release/gnome-inspector" "$BIN_PATH"

desktop_exec="${BIN_PATH//\\/\\\\}"
desktop_exec="${desktop_exec//\"/\\\"}"
desktop_exec="${desktop_exec//%/%%}"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Name=GNOME Inspector
Comment=Inspect and debug GNOME Shell
Exec="$desktop_exec"
Terminal=false
Type=Application
Categories=Development;Utility;
StartupNotify=true
EOF

bash "$ROOT_DIR/scripts/install-extension.sh"

echo
echo "GNOME Inspector installed."
echo "Application: $BIN_PATH"
echo "Launcher: $DESKTOP_FILE"
echo "Start it from the application menu or run: $BIN_PATH"
