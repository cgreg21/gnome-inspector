#!/usr/bin/env bash
set -Eeuo pipefail
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEST_DIR"' EXIT
mkdir -p "$TEST_DIR/bin"
touch "$TEST_DIR/gnome-inspector"
chmod +x "$TEST_DIR/gnome-inspector"
cat > "$TEST_DIR/bin/gnome-shell" <<'MOCK'
#!/usr/bin/env bash
echo "${MOCK_SHELL_VERSION:-GNOME Shell 51.0}"
MOCK
cat > "$TEST_DIR/bin/ldd" <<'MOCK'
#!/usr/bin/env bash
if [[ "${MOCK_MISSING_LIBRARY:-0}" == 1 ]]; then
  echo "libgtk-4.so.1 => not found"
else
  echo "libgtk-4.so.1 => /usr/lib/libgtk-4.so.1"
  echo "libadwaita-1.so.0 => /usr/lib/libadwaita-1.so.0"
fi
MOCK
chmod +x "$TEST_DIR/bin/"*
export PATH="$TEST_DIR/bin:$PATH"
bash "$ROOT_DIR/scripts/check-runtime.sh" "$TEST_DIR/gnome-inspector"
if MOCK_SHELL_VERSION="GNOME Shell 50.0" bash "$ROOT_DIR/scripts/check-runtime.sh" "$TEST_DIR/gnome-inspector"; then
  echo "GNOME Shell 50 should have been rejected." >&2
  exit 1
fi
if MOCK_MISSING_LIBRARY=1 bash "$ROOT_DIR/scripts/check-runtime.sh" "$TEST_DIR/gnome-inspector"; then
  echo "A missing runtime library should have been rejected." >&2
  exit 1
fi
echo "Runtime compatibility tests passed."
