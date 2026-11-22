#!/usr/bin/env bash
set -Eeuo pipefail
BINARY="${1:-}"
if [[ -z "$BINARY" || ! -x "$BINARY" ]]; then
  echo "Usage: $0 /path/to/gnome-inspector (executable required)." >&2
  exit 1
fi
if ! command -v gnome-shell >/dev/null 2>&1; then
  echo "GNOME Shell 51 is required." >&2
  exit 1
fi
SHELL_VERSION="$(gnome-shell --version)"
if [[ ! "$SHELL_VERSION" =~ GNOME\ Shell\ 51([.[:space:]]|$) ]]; then
  echo "Unsupported GNOME version: $SHELL_VERSION (GNOME Shell 51 required)." >&2
  exit 1
fi
if ! command -v ldd >/dev/null 2>&1; then
  echo "ldd is required to check runtime libraries." >&2
  exit 1
fi
if ! DEPENDENCIES="$(ldd "$BINARY" 2>&1)"; then
  echo "Could not inspect runtime dependencies:" >&2
  echo "$DEPENDENCIES" >&2
  exit 1
fi
if grep -Fq "not found" <<< "$DEPENDENCIES"; then
  echo "Missing application runtime libraries:" >&2
  grep -F "not found" <<< "$DEPENDENCIES" >&2
  echo "Install GTK 4 and libadwaita runtime packages." >&2
  exit 1
fi
echo "Runtime requirements satisfied: GNOME Shell 51 and shared libraries."
