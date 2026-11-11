# m6-parity-release — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-10 — Plan written

- Wrote `plan.md` (8 tasks) on the macOS host. Reference: cmux `Sources/` sparse clone at `43ce5e2` (2026-11-10).
- Found while planning:
  - The macOS cmuxOnly ancestry check already exists (`server.rs` `peer::parent_of`, libproc); removed from the M6 list.
  - cmux reads ports with libproc, not `lsof` (`Sources/ListeningPortLookup.swift`); libc 0.2.189 lacks `socket_fdinfo` and its constants, so the plan mirrors them (D1).
  - `setsid` and GNU `timeout` don't exist on macOS; the PTY exit test is ported with `trap '' HUP` (Task 3).
  - An install in `/usr/bin` would put `/usr/bin` first on every terminal's `PATH` (M5 D8), so the package installs to `/usr/lib/cmux-linux/` (D5).
- Spec gives no performance numbers; the plan sets them (D8). M5's deferred CLI commands stay out unless the owner adds them to the spec.
