# Changelog

## Unreleased

### Added

- Main-thread CPU utilization, cumulative CPU time and context-switch counters.
- GNOME Shell process resident memory, peak resident memory and thread count.
- A bounded 60-sample performance history containing the available metrics.

### Notes

- CPU time and context-switch counters describe the Shell main thread.
- Memory and thread-count metrics describe the Shell process.
- Metrics remain unavailable when the corresponding procfs data cannot be read.

## 0.1.0-alpha.1

First distributable alpha for GNOME Shell 51.

### Included

- Native GTK 4/libadwaita desktop application.
- GNOME Shell extension connected over the session D-Bus.
- Actor hierarchy inspection, selection, search and property details.
- Explicit editing of supported writable GObject properties.
- CSS inspection, actor style editing and user-owned custom stylesheet editor.
- GJS console, session diagnostics and basic scene/frame performance metrics.
- Per-user install and uninstall scripts, plus a prebuilt Linux archive.

### Known limitations in this release

- GNOME Shell 51 is the only supported Shell version.
- The release workflow currently builds Linux x86_64 archives on Ubuntu.
- Performance metrics do not yet include CPU or memory history.
- Actor and property inspection is intentionally bounded; some Shell objects may expose incomplete data.
- CSS editing is limited to the selected actor and the user-owned custom stylesheet; global theme files are not edited.
- The GJS console is a developer feature and executes code inside the GNOME Shell process.

This is an early alpha. Expect incomplete features, rough edges and breaking changes.
Back up important work and report reproducible issues with GNOME Shell logs.
