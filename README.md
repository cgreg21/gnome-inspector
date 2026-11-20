# GNOME Inspector

A native developer tool for inspecting and debugging GNOME Shell.

The first development target is GNOME Shell 51 on Linux. The project is intentionally
small at the start: a GTK4/libadwaita desktop application and a separate GNOME Shell
extension. The extension is the only component that will access Shell internals.

## Current status

This repository is at the foundation stage.

- [x] Rust + GTK4/libadwaita application skeleton
- [x] GNOME Shell 51 extension skeleton
- [ ] Application-to-extension transport (D-Bus)
- [ ] Actor tree and GObject inspection
- [ ] St theme/CSS inspection
- [ ] GJS console and diagnostics
- [ ] Network and performance inspection

The application currently presents the connection state as unavailable; it does not
claim to inspect a Shell process until a real transport and agent are implemented.

## Requirements

- Rust stable and Cargo
- GTK 4 development files
- libadwaita development files
- GNOME Shell 51 for the extension

On Debian/Ubuntu:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

## Build and run

```sh
cargo build
cargo run
```

Run the test suite and formatting checks:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Install the development extension

Copy the `extension/` directory to:

```
~/.local/share/gnome-shell/extensions/gnome-inspector@cgreg21/
```

Then log out and back in (or restart GNOME Shell where supported). Enable it with
GNOME Extensions or:

```sh
gnome-extensions enable gnome-inspector@cgreg21
```

The extension is currently a proof-of-life indicator only. It does not yet expose
private Shell objects or communicate with the desktop application.

## Design constraints

- Keep the implementation KISS: no speculative framework or plugin architecture.
- Rust owns the desktop application and general-purpose logic.
- GJS is limited to the GNOME Shell extension and Shell-specific APIs.
- Prefer GNOME/GLib APIs over custom infrastructure.
- Inspection is read-only by default; mutations must be explicit.
- A feature is not considered complete until it has meaningful tests.

## License

MIT. See [LICENSE](LICENSE).
