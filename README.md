# GNOME Inspector

A native developer tool for inspecting and debugging GNOME Shell.

The first development target is GNOME Shell 51 on Linux. The project starts with a
GTK4/libadwaita desktop application and a GNOME Shell extension. The extension is the
only component that accesses Shell-specific APIs.

## Current status

- [x] Rust + GTK4/libadwaita application
- [x] GNOME Shell 51 extension
- [x] Session D-Bus name and Ping handshake
- [ ] Actor tree and GObject inspection
- [ ] St theme/CSS inspection
- [ ] GJS console and diagnostics
- [ ] Network and performance inspection

The desktop application can verify that the extension is present and retrieve the
running Shell version. The inspector features listed above are not implemented yet.

## Requirements

- Rust stable and Cargo
- GTK 4 development files
- libadwaita development files
- GNOME Shell 51

On Debian/Ubuntu:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

## Build and run

```sh
cargo build
cargo run
```

Run checks:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Install the development extension

Copy `extension/` to:

```
~/.local/share/gnome-shell/extensions/gnome-inspector@cgreg21/
```

Then log out and back in (or restart GNOME Shell where supported), and enable it:

```sh
gnome-extensions enable gnome-inspector@cgreg21
```

Start GNOME Inspector and use **Retry connection** if the extension was enabled after
the application started.

## D-Bus interface

- Bus name: `io.github.cgreg21.GnomeInspector`
- Object path: `/io/github/cgreg21/GnomeInspector`
- Interface: `io.github.cgreg21.GnomeInspector`
- Method: `Ping() -> s` (the running GNOME Shell version)

## Design constraints

- Keep the implementation KISS: no speculative framework or plugin architecture.
- Rust owns the desktop application and general-purpose logic.
- GJS is limited to the GNOME Shell extension and Shell-specific APIs.
- Prefer GNOME/GLib APIs over custom infrastructure.
- Inspection is read-only by default; mutations must be explicit.
- A feature is not complete until it has meaningful tests.

## License

MIT. See [LICENSE](LICENSE).
