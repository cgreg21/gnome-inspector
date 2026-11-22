#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEST_DIR"' EXIT

export HOME="$TEST_DIR/home"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CONFIG_HOME="$HOME/.config"
export GNOME_INSPECTOR_BIN_DIR="$HOME/.local/bin"
export CARGO_TARGET_DIR="$TEST_DIR/build"
mkdir -p "$HOME" "$TEST_DIR/bin" "$XDG_CONFIG_HOME/gnome-inspector"

cat > "$TEST_DIR/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
set -Eeuo pipefail
mkdir -p "$CARGO_TARGET_DIR/release"
printf '#!/usr/bin/env sh\necho mock-gnome-inspector\n' > "$CARGO_TARGET_DIR/release/gnome-inspector"
chmod +x "$CARGO_TARGET_DIR/release/gnome-inspector"
MOCK

cat > "$TEST_DIR/bin/gnome-extensions" <<'MOCK'
#!/usr/bin/env bash
set -Eeuo pipefail
case "${1:-}" in
    list) echo "gnome-inspector@cgreg21" ;;
    enable|disable|info) exit 0 ;;
    *) exit 2 ;;
esac
MOCK

chmod +x "$TEST_DIR/bin/cargo" "$TEST_DIR/bin/gnome-extensions"
export PATH="$TEST_DIR/bin:$PATH"

echo "preserve this file" > "$XDG_CONFIG_HOME/gnome-inspector/custom.css"

bash "$ROOT_DIR/scripts/install.sh"

[[ -x "$GNOME_INSPECTOR_BIN_DIR/gnome-inspector" ]]
[[ -f "$XDG_DATA_HOME/applications/io.github.cgreg21.GnomeInspector.desktop" ]]
[[ -f "$XDG_DATA_HOME/gnome-shell/extensions/gnome-inspector@cgreg21/metadata.json" ]]
grep -Fq "Exec=\"$GNOME_INSPECTOR_BIN_DIR/gnome-inspector\"" \
    "$XDG_DATA_HOME/applications/io.github.cgreg21.GnomeInspector.desktop"

bash "$ROOT_DIR/scripts/uninstall.sh"

[[ ! -e "$GNOME_INSPECTOR_BIN_DIR/gnome-inspector" ]]
[[ ! -e "$XDG_DATA_HOME/applications/io.github.cgreg21.GnomeInspector.desktop" ]]
[[ ! -e "$XDG_DATA_HOME/gnome-shell/extensions/gnome-inspector@cgreg21" ]]
[[ "$(cat "$XDG_CONFIG_HOME/gnome-inspector/custom.css")" == "preserve this file" ]]

echo "Installation and uninstallation smoke tests passed."
