# M15 hardening — Spec

**Goal:** close the known bugs and data-loss gaps from M3–M14, and make the "never
clicked" UI checkable by script, so the open-risks list in `STATUS.md` shrinks instead
of growing.

## Scope
- **Scripted clicks:** a test-only socket command `debug.mouse.simulate` (like
  `debug.shortcut.simulate`) that sends move/down/up/scroll at window coordinates through
  gpui's `Window::dispatch_event`. Used below to click through the M7–M14 surfaces.
- **No lost state:**
  - SIGTERM/SIGHUP (logout, `systemctl --user stop`) runs the same save as quit.
  - A build without `browser` keeps saved browser tabs instead of dropping them.
  - Quitting from the main window commits a half-typed Settings field.
  - `.cmux-bak` is written once (first edit), not overwritten each run.
- **Dock on the socket:** Dock terminals get `CMUX_*` env and surface refs, so `cmux
  notify`, the Feed hook and OSC notifications work there and CLI commands can target them.
- **CLI/socket minors (M5, M7):** no panic on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` or
  non-UTF-8 args; a request that timed out on the client is dropped, not run later; a
  second app start on the same socket fails cleanly (flock); hook stdin read has a timeout;
  `CMUX_LINUX` only acts with our socket path. Missing CLI tests from the M7 review added.
- **Sidebar minors (M3):** keyboard workspace move scrolls into view; long rename keeps the
  caret visible; git dirty `*` like cmux.
- **Scrollback:** Ghostty `scrollback-limit` sets the per-terminal limit (default stays 10 MB).
- **Click-through:** every button/menu/control listed as "unclicked" in `STATUS.md` (Feed
  cards, group header/menu, Dock mode bar, trust prompt, Settings controls, popover, context
  menu, title bar double-click, browser buttons) gets one scripted click check on KDE.
  Bugs found are fixed in this milestone if small, else recorded.
- `STATUS.md` risks list rewritten to what is still true.

## Out of scope
- Real-hardware-only checks: physical touchpad rate, real IME (fcitx/ibus), real Claude
  Code runs, `makepkg -si` with sudo, blur by eye. They stay owner checks.
- ssh reconnect loop, render/perf work (per-cell font clone, blink wakeups), gpui upgrade.
- New features.

## Exit criteria
1. `debug.mouse.simulate` clicks a sidebar row and the workspace switches (scripted test).
2. `kill -TERM` on the app, restart: the last layout change made 1 s before the kill is
   restored.
3. `cmux notify` from a Dock terminal shows a notification; `cmux list-panes` lists Dock
   panes.
4. Each CLI/socket minor above has a test that failed before and passes after.
5. Click-through script runs on KDE and its results (pass/fail per control) are in the
   journal; every fail is fixed or listed in `STATUS.md`.
