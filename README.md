# GNOME Inspector

A native developer tool for inspecting and debugging GNOME Shell.

The first development target is GNOME Shell 51 on Linux. The project uses a GTK4/libadwaita
desktop application and a GNOME Shell extension. The extension is the only component
that accesses Shell-specific APIs.

## Current status

- [x] Rust + GTK4/libadwaita desktop application
- [x] GNOME Shell 51 extension
- [x] Session D-Bus name and version handshake
- [x] Bounded Shell actor-tree snapshot (JSON)
- [x] Native application shell with module navigation
- [x] Actor hierarchy navigation and property JSON inspection
- [ ] Dedicated property editor and live actor selection
- [x] Actor CSS classes, pseudo-classes and inline-style snapshots
- [x] Explicit per-actor inline CSS editing
- [ ] Theme stylesheet discovery and global theme editing
- [x] Interactive GJS console (explicit code execution)
- [x] GNOME Shell session diagnostics
- [x] Shell scene capture metrics (duration, actor count, stage and monitors)
- [ ] Frame timing, memory and CPU history

The actor snapshot is read-only and deliberately bounded to 1,000 actors and 12 levels.
Its response includes an actor count, a truncation flag, and an `incomplete` flag when
an actor property or child list cannot be read. Per-actor `propertyErrors`, `childrenError`,
and `actorError` fields explain failures. Invalid geometry uses safe fallback values, and
repeated actor references are reported instead of being traversed again. The desktop app currently provides a navigable workspace for Overview, Actors, CSS and themes,
GJS Console, Diagnostics, and Performance. The actor module displays the hierarchy as a selectable list and shows the selected actor's properties as formatted JSON. The CSS module retrieves style classes, pseudo-classes and inline styles over D-Bus, presents a selectable actor list, and supports explicit inline-style updates for the selected actor. Updates are limited to 4,096 characters and resolve actors through a bounded child-index path. Theme stylesheet discovery and global theme editing remain to be implemented. The performance module provides live scene-capture duration, actor count, stage dimensions and monitor count; frame timing and resource history remain to be implemented. The GJS console executes user-supplied code in the connected Shell process; it is intentionally powerful and must only be used with trusted code. The Diagnostics module retrieves the Shell version, session/display environment, stage dimensions, monitor count, and extension metadata from the live Shell agent.

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

Run checks and diagnostics:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
RUST_BACKTRACE=full cargo test --all-targets --all-features -- --nocapture
cargo test --doc --all-features
cargo clippy --all-targets --all-features -- -D warnings
node --test --test-reporter=spec extension/*.test.js
shellcheck scripts/test-gnome-shell-51.sh
```

CI additionally runs:

- **cargo-nextest** for the Rust test suite.
- **cargo-llvm-cov** and publishes the LCOV coverage report.
- **cargo-audit** against the RustSec advisory database.
- **cargo-machete** to detect unused direct dependencies.
- **cargo tree --duplicates** to expose duplicate dependency versions.
- JavaScript syntax and extension metadata validation.
- ShellCheck and actionlint for the CI/integration scripts.
- A real GNOME Shell 51 extension integration test (see below).

## GNOME Shell 51 virtual test session

The `gnome-shell-51-virtual-session` CI job uses Fedora 45, whose packaged GNOME Shell
is version 51. It launches an isolated, headless GNOME Shell process inside a private
D-Bus session. This is a virtual Shell runtime, not a full virtual-machine boot or a
complete GDM login session.

The test installs and enables the extension in a temporary home directory, then checks:

1. The Shell process is actually GNOME Shell 51.
2. The extension loads and owns its D-Bus service.
3. `Ping` returns the Shell version.
4. `GetActorTree` returns the expected bounded JSON snapshot.
5. `GetStyleTree`, `GetDiagnostics`, `EvaluateGjs`, and `GetPerformance` respond.\n6. Invalid CSS actor paths are rejected.\n7. The Shell log contains no GJS extension errors or criticals.

The mocked logind service log is uploaded alongside the Shell log for diagnostics.

The job uploads `gnome-shell-51.log` even when the test fails, so startup and extension
errors remain available for diagnosis. Run the same test locally on Fedora 45 with:

```sh
dbus-run-session -- bash scripts/test-gnome-shell-51.sh
```

The CI session uses the headless Mutter backend; it validates the Shell process and
extension/D-Bus integration, but it does not replace manual testing on a graphical
Wayland session with real input and display hardware.

## Install the development extension

Copy `extension/` to:

```
~/.local/share/gnome-shell/extensions/gnome-inspector@cgreg21/
```

Then log out and back in (or restart GNOME Shell where supported), and enable it:

```sh
gnome-extensions enable gnome-inspector@cgreg21
```

Start GNOME Inspector. Use **Retry connection** if the extension was enabled after
the application started, and **Refresh actor tree** to request a fresh snapshot.

## D-Bus interface

- Bus name: `io.github.cgreg21.GnomeInspector`
- Object path: `/io/github/cgreg21/GnomeInspector`
- Interface: `io.github.cgreg21.GnomeInspector`
- `Ping() -> s`: running GNOME Shell version
- `GetActorTree() -> s`: bounded JSON snapshot of the Shell stage and its descendants,
  including truncation, incomplete-capture status, and per-property failure details

## Design constraints

- Keep the implementation KISS: no speculative framework or plugin architecture.
- Rust owns the desktop application and general-purpose logic.
- GJS is limited to the GNOME Shell extension and Shell-specific APIs.
- Prefer GNOME/GLib APIs over custom infrastructure.
- Inspection is read-only by default; mutations must be explicit.
- A feature is not complete until it has meaningful tests.

## License

MIT. See [LICENSE](LICENSE).
