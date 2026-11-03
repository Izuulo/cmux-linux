# m2-splits-tabs — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-03 — Tasks 1–10 landed; exit checks pending

- Tasks 1–10 are committed on `m2-splits-tabs`; CI green at `3b6e338` (build, test, clippy, fmt on ubuntu-24.04). Plan checkboxes ticked after the fact, one commit per task.
- Left unticked: every "Run it" step (Tasks 2, 6, 7, 8, 9, 10). The fix commits after Tasks 7 and 8 (pane background, drag release, tab × hover) suggest they were run, but no results were recorded. Redo them as part of Task 11 Step 4.
- Extra commits beyond the plan: README configuration section (Ghostty config, `[keybindings]`, KWin Super conflicts); cwd falls back to `/proc`, failed shell spawn no longer panics, tab × ignores double-click.
- Task 12 Step 2 (design.md `toml` dependency) done early; it doesn't depend on Task 11.
- Not yet possible on macOS: `install-zig.sh` is Linux-only and the PTY uses pidfd, so Task 11 waits for the KDE desktop.
- Exit 3 option: a Mac with cmux installed can supply a real `cmux-mac-splits.png` (same 2 × 2 layout) instead of cmux's published screenshot.
