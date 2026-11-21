#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p screenshots

gjs scripts/mock-inspector-dbus.js > /tmp/gnome-inspector-mock.log 2>&1 &
mock_pid=$!
app_pid=""

cleanup() {
    if [[ -n "$app_pid" ]]; then
        kill "$app_pid" 2>/dev/null || true
    fi
    kill "$mock_pid" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 20); do
    if gdbus call --session --dest io.github.cgreg21.GnomeInspector.Shell \
        --object-path /io/github/cgreg21/GnomeInspector \
        --method io.github.cgreg21.GnomeInspector.Ping >/dev/null 2>&1; then
        break
    fi
    sleep 0.5
done

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
    local y="$1"
    xdotool mousemove --window "$window_id" 100 "$y" click 1
    sleep 1
}

capture overview

click_page 108
xdotool mousemove --window "$window_id" 1200 170 click 1
sleep 1
capture actors

click_page 144
xdotool mousemove --window "$window_id" 1200 170 click 1
sleep 1
xdotool mousemove --window "$window_id" 350 250 click 1
sleep 0.5
capture css-themes

click_page 180
xdotool mousemove --window "$window_id" 320 170 click 1
sleep 1
capture gjs-console

click_page 216
xdotool mousemove --window "$window_id" 350 170 click 1
sleep 1
capture diagnostics

click_page 252
xdotool mousemove --window "$window_id" 350 390 click 1
sleep 1
capture performance
