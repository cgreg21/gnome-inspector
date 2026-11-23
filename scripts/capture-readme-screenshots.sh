#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p screenshots

gjs -m scripts/mock-inspector-dbus.js > /tmp/gnome-inspector-mock.log 2>&1 &
mock_pid=$!
app_pid=""

cleanup() {
    if [[ -n "$app_pid" ]]; then
        kill "$app_pid" 2>/dev/null || true
    fi
    kill "$mock_pid" 2>/dev/null || true
}
trap cleanup EXIT

mock_ready=false
for _ in $(seq 1 20); do
    if gdbus call --session --dest io.github.cgreg21.GnomeInspector.Shell \
        --object-path /io/github/cgreg21/GnomeInspector \
        --method io.github.cgreg21.GnomeInspector.Ping >/dev/null 2>&1; then
        mock_ready=true
        break
    fi
    sleep 0.5
done
if [[ "$mock_ready" != true ]]; then
    echo "Screenshot D-Bus mock did not become ready" >&2
    cat /tmp/gnome-inspector-mock.log >&2
    exit 1
fi

echo "Building GNOME Inspector for screenshots"
cargo build --release
echo "Starting the GTK application"
./target/release/gnome-inspector &
app_pid=$!

window_id=""
for _ in $(seq 1 30); do
    window_id="$(xdotool search --onlyvisible --name '^GNOME Inspector$' 2>/dev/null | head -n 1 || true)"
    if [[ -n "$window_id" ]]; then
        break
    fi
    sleep 0.5
done
if [[ -z "$window_id" ]]; then
    echo "GNOME Inspector window did not appear" >&2
    cat /tmp/gnome-inspector-mock.log >&2
    exit 1
fi

sleep 2
echo "Capturing application window $window_id"
capture() {
    local name="$1"
    echo "Capturing $name"
    xwd -silent -id "$window_id" -out "/tmp/$name.xwd"
    convert "/tmp/$name.xwd" "screenshots/$name.png"
    test -s "screenshots/$name.png"
}

click_page() {
    local x="$1"
    xdotool mousemove --window "$window_id" "$x" 22 click 1
    sleep 1
}

capture overview

click_page 283
xdotool mousemove --window "$window_id" 1130 168 click 1
sleep 2
capture actors
xdotool mousemove --window "$window_id" 130 292 click 1
sleep 1
capture actors-selected

click_page 437
xdotool mousemove --window "$window_id" 1164 80 click 1
sleep 2
xdotool mousemove --window "$window_id" 935 80 click 1
sleep 1
capture css-inspector-tree

xdotool mousemove --window "$window_id" 100 206 click 1
sleep 1
capture css-inspector-selection

xdotool mousemove --window "$window_id" 100 365 click 1
sleep 1
capture css-inspector-deep-selection
capture css-themes

click_page 586
xdotool mousemove --window "$window_id" 320 190 click 1
sleep 1
capture gjs-console

click_page 744
xdotool mousemove --window "$window_id" 350 170 click 1
sleep 1
capture diagnostics

click_page 900
xdotool mousemove --window "$window_id" 350 410 click 1
sleep 2
capture performance
