#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
UUID="gnome-inspector@cgreg21"
BIN_DIR="${GNOME_INSPECTOR_BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_SOURCE="$ROOT_DIR/bin/gnome-inspector"
EXTENSION_SOURCE="$ROOT_DIR/extension"
EXTENSION_TARGET="$DATA_DIR/gnome-shell/extensions/$UUID"
DESKTOP_DIR="$DATA_DIR/applications"
DESKTOP_FILE="$DESKTOP_DIR/io.github.cgreg21.GnomeInspector.desktop"

if [[ ! -x "$BIN_SOURCE" || ! -f "$EXTENSION_SOURCE/metadata.json" ]]; then
    echo "This archive is incomplete; extract it before running install.sh." >&2
    exit 1
fi
if ! command -v gnome-extensions >/dev/null 2>&1; then
    echo "gnome-extensions is required to install the GNOME Shell agent." >&2
    exit 1
fi

bash "$ROOT_DIR/check-runtime.sh" "$BIN_SOURCE"

install -d "$BIN_DIR" "$DESKTOP_DIR" "$(dirname -- "$EXTENSION_TARGET")"
install -m 755 "$BIN_SOURCE" "$BIN_DIR/gnome-inspector"
desktop_exec="${BIN_DIR//\\/\\\\}"
desktop_exec="${desktop_exec//\"/\\\"}"
desktop_exec="${desktop_exec//%/%%}"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Name=GNOME Inspector
Comment=Inspect and debug GNOME Shell
Exec="$desktop_exec/gnome-inspector"
Terminal=false
Type=Application
Categories=Development;Utility;
StartupNotify=true
EOF

rm -rf -- "$EXTENSION_TARGET"
cp -a -- "$EXTENSION_SOURCE" "$EXTENSION_TARGET"

if ! LISTED_EXTENSIONS="$(gnome-extensions list)"; then
    echo "Files installed, but GNOME extensions could not be listed." >&2
    exit 1
fi
if grep -Fxq "$UUID" <<< "$LISTED_EXTENSIONS"; then
    gnome-extensions enable "$UUID"
else
    echo "GNOME Shell has not discovered the extension yet; log out and back in."
fi

echo "GNOME Inspector installed in $BIN_DIR."
