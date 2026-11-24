# Roadmap

This roadmap tracks the planned work for GNOME Inspector. It is intentionally
short and ordered by priority; completed items are kept as a compact record.
Update this file in the same change whenever implementation status or priorities
change.

## Current status

- **Stage:** alpha, targeting GNOME Shell 51.
- **Active work:** review metric lifecycle and unavailable values, then improve peak/current summaries.
- **Scope:** keep the desktop app in Rust/GTK4/libadwaita and use GJS only for
  GNOME Shell-specific APIs.
- **Principle:** KISS. Prefer small, testable changes and existing platform APIs.

## Completed

- [x] Native GTK4/libadwaita application and GNOME Shell extension connected over
  session D-Bus.
- [x] Actor tree browsing, search, selection and supported property inspection/editing.
- [x] CSS/style inspection, supported live edits and user-owned custom stylesheet.
- [x] Computed style and layout information where exposed by GNOME Shell.
- [x] GJS console and session/extension diagnostics.
- [x] Basic scene/frame performance metrics and rendered-frame-rate history.
- [x] Main-thread CPU utilization, cumulative CPU time and context-switch counters.
- [x] Shell process resident memory, peak resident memory and thread count.
- [x] Bounded performance history (60 samples) and procfs parsing tests.

## Next priorities

### 1. Performance monitoring

- [ ] Review metric lifecycle, unavailable values and behavior when procfs cannot
  be read.
- [x] Add native CPU, memory and FPS history charts using the existing bounded samples; missing values create visible gaps.
- [ ] Show useful peaks/current values and handle gaps without inventing data.
- [ ] Add pause/resume and reset controls if they fit the existing data flow.
- [ ] Consider export only after the in-app monitoring workflow is stable.

### 2. Inspector usability

- [ ] Improve actor-tree filtering and navigation for large hierarchies.
- [ ] Make property changes and their errors easier to understand.
- [ ] Improve CSS rule provenance and conflict visibility where the Shell APIs
  provide reliable information.
- [ ] Keep computed values and box-model details aligned with the actual data
  available from GNOME Shell.

### 3. Console and diagnostics

- [ ] Improve console history and error presentation.
- [ ] Add completion only where it can be implemented reliably without a
  heavyweight runtime dependency.
- [ ] Expand extension diagnostics and actionable error messages.

### 4. Advanced inspection

- [ ] Explore actor snapshots/diffs and hierarchy-change tracking.
- [ ] Explore event/signal inspection only where supported APIs make it safe and
  understandable.
- [ ] Consider a visual actor picker and workflow refinements based on testing.

### 5. Compatibility and distribution

- [ ] Test against additional GNOME Shell versions before declaring support.
- [ ] Improve installation and upgrade guidance based on real-world testing.
- [ ] Keep release artifacts reproducible and document supported platforms.
- [ ] Prepare a beta checklist covering stability, accessibility and diagnostics.

## Maintenance rules

- Update the relevant checklist in this file in the same pull request as each
  feature, fix or scope change.
- Mark an item complete only when its implementation and required checks pass.
- Add newly discovered work under the appropriate priority; reorder only when
  there is a concrete reason.
- Keep the roadmap factual: do not promise APIs or metrics that GNOME Shell does
  not expose.
- Apply KISS: avoid unnecessary abstractions, dependencies and speculative work.
- Do not treat a planned item as complete merely because a partial prototype exists.

## Verification

A change is complete only after the applicable tests and all required CI jobs
pass on the exact commit being reviewed. The current project CI includes Rust
checks, extension tests/static analysis, README screenshot capture and the
GNOME Shell 51 virtual-session integration test.
