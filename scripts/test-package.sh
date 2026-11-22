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
export GNOME_INSPECTOR_DIST_DIR="$TEST_DIR/dist"
mkdir -p "$HOME" "$TEST_DIR/bin" "$XDG_CONFIG_HOME/gnome-inspector"

cat > "$TEST_DIR/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
set -Eeuo pipefail
mkdir -p "$CARGO_TARGET_DIR/release"
printf '#!/usr/bin/env sh\necho packaged-gnome-inspector\n' > "$CARGO_TARGET_DIR/release/gnome-inspector"
chmod +x "$CARGO_TARGET_DIR/release/gnome-inspector"
MOCK
cat > "$TEST_DIR/bin/gnome-shell" <<'MOCK'
#!/usr/bin/env bash
echo "GNOME Shell 51.0"
MOCK
cat > "$TEST_DIR/bin/ldd" <<'MOCK'
#!/usr/bin/env bash
echo "libgtk-4.so.1 => /usr/lib/libgtk-4.so.1"
MOCK
cat > "$TEST_DIR/bin/gnome-extensions" <<'MOCK'
#!/usr/bin/env bash
set -Eeuo pipefail
case "${1:-}" in
    list) echo "gnome-inspector@cgreg21" ;;
    enable|disable) exit 0 ;;
    *) exit 2 ;;
esac
MOCK
chmod +x "$TEST_DIR/bin/cargo" "$TEST_DIR/bin/gnome-shell" "$TEST_DIR/bin/ldd" "$TEST_DIR/bin/gnome-extensions"
export PATH="$TEST_DIR/bin:$PATH"

echo "keep user CSS" > "$XDG_CONFIG_HOME/gnome-inspector/custom.css"
bash "$ROOT_DIR/scripts/package-alpha.sh"
ARCHIVE="$(find "$GNOME_INSPECTOR_DIST_DIR" -maxdepth 1 -name '*.tar.gz' -print -quit)"
[[ -n "$ARCHIVE" ]]
mkdir "$TEST_DIR/extracted"
tar -xzf "$ARCHIVE" -C "$TEST_DIR/extracted"
BUNDLE="$(find "$TEST_DIR/extracted" -mindepth 1 -maxdepth 1 -type d -print -quit)"
[[ -x "$BUNDLE/bin/gnome-inspector" ]]
[[ -f "$BUNDLE/extension/metadata.json" ]]
[[ -x "$BUNDLE/check-runtime.sh" ]]
"$BUNDLE/bin/gnome-inspector" | grep -Fxq "packaged-gnome-inspector"

bash "$BUNDLE/install.sh"
[[ -x "$GNOME_INSPECTOR_BIN_DIR/gnome-inspector" ]]
[[ -f "$XDG_DATA_HOME/gnome-shell/extensions/gnome-inspector@cgreg21/metadata.json" ]]
[[ -f "$XDG_DATA_HOME/applications/io.github.cgreg21.GnomeInspector.desktop" ]]

bash "$BUNDLE/uninstall.sh"
[[ ! -e "$GNOME_INSPECTOR_BIN_DIR/gnome-inspector" ]]
[[ ! -e "$XDG_DATA_HOME/gnome-shell/extensions/gnome-inspector@cgreg21" ]]
[[ ! -e "$XDG_DATA_HOME/applications/io.github.cgreg21.GnomeInspector.desktop" ]]
[[ "$(cat "$XDG_CONFIG_HOME/gnome-inspector/custom.css")" == "keep user CSS" ]]

echo "Alpha archive packaging and installation tests passed."
