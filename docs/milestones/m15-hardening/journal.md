# M15 hardening — Journal

## 2026-11-14
- Milestone picked by owner (hardening over release/upstream catch-up). Found during spec:
  groups and Dock are already persisted (STATUS was stale); SIGTERM had no handler, so
  logout skipped the quit save; gpui 0.2.2 `Window::dispatch_event` is public, so scripted
  clicks need no gpui patch.
- Task 1, scripted clicks: `Window::dispatch_event` returns gpui's `pub(crate)`
  `DispatchEventResult`, so Rust's type privacy refuses the call from outside gpui. Owner
  chose to vendor gpui 0.2.2 (`vendor/gpui`, src only, `[patch.crates-io]`, excluded from
  the workspace) with that struct made `pub` (`vendor/gpui/PATCHED.md`). Clippy now shows
  two of gpui's own warnings (path deps are not lint-capped); they do not fail `-D warnings`.
  `debug.mouse.simulate {x, y, kind: click|move|drag|down|up|scroll, button, click_count, dy}`.
  Scroll sign (`dy` + = down) not yet checked against a real wheel.
- Exit 1 checked on KDE (dev build, `XDG_STATE_HOME` under `$XDG_RUNTIME_DIR`, `allowAll`,
  bash wrapper shell): three workspaces, clicks at x 80 switch to row 1 at y 40–100, row 2
  at y 120–140, row 3 at y 170–200; y 20 (above the list) does nothing.
- Task 2, SIGTERM/SIGHUP: `quit_signal.rs` (`SA_RESETHAND` handler → socketpair → thread →
  `cx.quit()`). Exit 2 checked: relaunch restored the earlier split; split down, 1 s,
  `kill -TERM`: app exits, `session.json` mtime equals the kill time to the ms and holds
  both splits.
