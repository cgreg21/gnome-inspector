#!/usr/bin/env bash
set -euo pipefail

UUID="gnome-inspector@cgreg21"
SOURCE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/extension"
TARGET_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$UUID"

if ! command -v gnome-extensions >/dev/null 2>&1; then
    echo "gnome-extensions is required. Install GNOME Shell's extension tools first." >&2
    exit 1
fi

if [[ ! -f "$SOURCE_DIR/metadata.json" || ! -f "$SOURCE_DIR/extension.js" ]]; then
    echo "The extension source is incomplete: $SOURCE_DIR" >&2
    exit 1
fi

mkdir -p "$(dirname -- "$TARGET_DIR")"
rm -rf -- "$TARGET_DIR"
cp -a -- "$SOURCE_DIR" "$TARGET_DIR"

echo "Installed $UUID to $TARGET_DIR"

if ! LISTED_EXTENSIONS="$(gnome-extensions list)"; then
    echo "Files installed, but GNOME extensions could not be listed." >&2
    exit 1
fi

if ! grep -Fxq "$UUID" <<< "$LISTED_EXTENSIONS"; then
    echo "GNOME Shell has not discovered the new extension yet."
    echo "Log out and back in, then run: gnome-extensions enable $UUID"
    exit 0
fi

if gnome-extensions enable "$UUID"; then
    echo "Extension activation requested."
else
    echo "Installation succeeded, but activation failed." >&2
    echo "Check GNOME Shell 51 and run: gnome-extensions enable $UUID" >&2
    exit 1
fi

gnome-extensions info "$UUID"
