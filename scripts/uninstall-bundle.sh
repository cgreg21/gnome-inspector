#!/usr/bin/env bash
set -Eeuo pipefail

UUID="gnome-inspector@cgreg21"
BIN_DIR="${GNOME_INSPECTOR_BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"

if command -v gnome-extensions >/dev/null 2>&1; then
    gnome-extensions disable "$UUID" >/dev/null 2>&1 || true
fi
rm -f -- "$BIN_DIR/gnome-inspector"
rm -f -- "$DATA_DIR/applications/io.github.cgreg21.GnomeInspector.desktop"
rm -rf -- "$DATA_DIR/gnome-shell/extensions/$UUID"

echo "GNOME Inspector removed. User configuration was preserved."
