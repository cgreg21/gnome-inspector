#!/usr/bin/env bash
set -Eeuo pipefail

UUID="gnome-inspector@cgreg21"
LOG_FILE="${GNOME_TEST_LOG:-$PWD/gnome-shell-51.log}"
HOME_DIR="$(mktemp -d)"
RUNTIME_DIR="$(mktemp -d)"
SHELL_PID=""

mkdir -p "$(dirname "$LOG_FILE")"
chmod 700 "$RUNTIME_DIR"
export HOME="$HOME_DIR"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_RUNTIME_DIR="$RUNTIME_DIR"
export GSETTINGS_BACKEND=memory
export G_MESSAGES_DEBUG=all
export SHELL_DEBUG=all
export LIBGL_ALWAYS_SOFTWARE=1

cleanup() {
    if [[ -n "$SHELL_PID" ]] && kill -0 "$SHELL_PID" 2>/dev/null; then
        kill "$SHELL_PID" 2>/dev/null || true
        wait "$SHELL_PID" 2>/dev/null || true
    fi
    rm -rf "$HOME_DIR" "$RUNTIME_DIR"
}
trap cleanup EXIT

version="$(gnome-shell --version)"
echo "Testing $version"
if [[ ! "$version" =~ GNOME\ Shell\ 51([.]|$) ]]; then
    echo "Expected GNOME Shell 51, got: $version" >&2
    exit 1
fi

extension_dir="$XDG_DATA_HOME/gnome-shell/extensions/$UUID"
mkdir -p "$extension_dir"
cp -R extension/. "$extension_dir/"

gnome-shell --headless --virtual-monitor 1280x720 >"$LOG_FILE" 2>&1 &
SHELL_PID=$!

ready=false
for _ in $(seq 1 30); do
    if ! kill -0 "$SHELL_PID" 2>/dev/null; then
        echo "GNOME Shell exited during startup" >&2
        cat "$LOG_FILE" >&2
        exit 1
    fi
    if timeout 3s gdbus call --session \
        --dest org.gnome.Shell \
        --object-path /org/gnome/Shell \
        --method org.freedesktop.DBus.Peer.Ping >/dev/null 2>&1; then
        ready=true
        break
    fi
    sleep 1
done

if [[ "$ready" != true ]]; then
    echo "GNOME Shell did not export its D-Bus interface" >&2
    cat "$LOG_FILE" >&2
    exit 1
fi

enabled="$(timeout 5s gdbus call --session \
    --dest org.gnome.Shell.Extensions \
    --object-path /org/gnome/Shell/Extensions \
    --method org.gnome.Shell.Extensions.EnableExtension "$UUID")"
if [[ "$enabled" != *"true"* ]]; then
    echo "Could not enable the GNOME Inspector extension: $enabled" >&2
    cat "$LOG_FILE" >&2
    exit 1
fi

ping=""
for _ in $(seq 1 30); do
    ping="$(timeout 3s gdbus call --session \
        --dest io.github.cgreg21.GnomeInspector \
        --object-path /io/github/cgreg21/GnomeInspector \
        --method io.github.cgreg21.GnomeInspector.Ping 2>/dev/null || true)"
    if [[ -n "$ping" ]]; then
        break
    fi
    sleep 1
done

if [[ -z "$ping" ]]; then
    echo "GNOME Inspector did not acquire its D-Bus name" >&2
    cat "$LOG_FILE" >&2
    exit 1
fi

echo "D-Bus Ping: $ping"
if [[ "$ping" != *"51."* ]]; then
    echo "The extension did not report GNOME Shell 51" >&2
    exit 1
fi

tree="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.GetActorTree)"

grep -q 'actorCount' <<<"$tree"
grep -q 'truncated' <<<"$tree"
grep -q 'root' <<<"$tree"
echo "Actor tree D-Bus snapshot: OK"

styles="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.GetStyleTree)"

grep -q 'actorCount' <<<"$styles"
grep -q 'styleClass' <<<"$styles"
grep -q 'root' <<<"$styles"
echo "Actor CSS style D-Bus snapshot: OK"

computed_style="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.GetActorComputedStyle '')"

grep -q 'available' <<<"$computed_style"
echo "Computed actor CSS style D-Bus response: OK"

theme_info="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.GetThemeInfo)"

grep -q 'available' <<<"$theme_info"
grep -q 'themeStylesheet' <<<"$theme_info"
grep -q 'customStylesheets' <<<"$theme_info"
echo "Active GNOME theme stylesheet discovery: OK"

class_update="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.SetActorStyleClass 'invalid/path' 'inspector-test')"

grep -q 'false' <<<"$class_update"
echo "Invalid CSS class actor path rejected: OK"

diagnostics="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.GetDiagnostics)"

grep -q 'shellVersion' <<<"$diagnostics"
grep -q 'monitorCount' <<<"$diagnostics"
grep -q 'stage' <<<"$diagnostics"
echo "GNOME Shell diagnostics D-Bus response: OK"

evaluation="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.EvaluateGjs '1 + 1')"

grep -q '2' <<<"$evaluation"
echo "GJS console evaluation: OK"

sleep 2
performance="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.GetPerformance)"

grep -q 'captureDurationUs' <<<"$performance"
grep -q 'actorCount' <<<"$performance"
grep -q 'monitorCount' <<<"$performance"
grep -q 'renderedFrames' <<<"$performance"
grep -q 'framesPerSecond' <<<"$performance"
grep -q 'performanceHistory' <<<"$performance"
echo "GNOME Shell rolling rendered-frame-rate history: OK"

style_update="$(timeout 10s gdbus call --session \
    --dest io.github.cgreg21.GnomeInspector \
    --object-path /io/github/cgreg21/GnomeInspector \
    --method io.github.cgreg21.GnomeInspector.SetActorInlineStyle 'invalid/path' 'color: red;')"

grep -q 'false' <<<"$style_update"
echo "Invalid CSS actor path rejected: OK"

if grep -E 'JS ERROR|Gjs-CRITICAL|Error while loading.*extension|Failed to load extension' "$LOG_FILE"; then
    echo "GNOME Shell reported an extension/runtime error" >&2
    exit 1
fi

echo "GNOME Shell 51 headless session integration test passed."
