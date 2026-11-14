# M15 hardening — Plan

One task per spec bullet, each with its own test, in this order (later tasks use the
earlier ones):

1. **`debug.mouse.simulate`** (`window_view/control.rs`): `{x, y, kind: move|down|up|click|
   scroll, button, click_count, dy}` → `PlatformInput` through `Window::dispatch_event`,
   deferred like `debug.shortcut.simulate`, same window choice (Settings when in front).
   Check: script clicks a sidebar row, `workspace.current` changes (exit 1).
2. **SIGTERM/SIGHUP save** (`quit_signal.rs`, unix): `SA_RESETHAND` handler writes one
   byte to a socketpair; a thread blocks on the read end and wakes a foreground task that
   calls `cx.quit()`, so the existing `on_app_quit` save runs. A second signal kills.
   Check: split, wait 1 s, `kill -TERM`, restart, split restored (exit 2).
3. **Saved browser tabs in no-`browser` builds**, **Settings field commit on quit**,
   **`.cmux-bak` once**: each a unit test where the code allows, else a scripted check.
4. **Dock on the socket**: Dock surfaces get refs and `CMUX_*` env; snapshot lists them;
   notifications route to them (exit 3).
5. **CLI/socket minors**: one failing test first per item (exit 4).
6. **Sidebar minors** and **`scrollback-limit`**.
7. **Click-through script** (`scripts/click-through.sh`) over the STATUS "unclicked" list;
   results in the journal (exit 5).
8. `STATUS.md` risks rewritten.
