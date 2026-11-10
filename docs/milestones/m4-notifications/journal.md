# m4-notifications — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-10 — Research and plan

- Branch `m4-notifications` from `master`. The M2/M3 desktop exit checks are still pending; the owner chose to start M4 meanwhile (this session ran on the macOS host).
- Research: `research-cmux.md` (cmux @ 21dcc2b, cited `rn §n`). Its two UNVERIFIED points were settled from the vendored Ghostty fork (MIT): OSC 9 ConEmu rules in `osc/parsers/osc9.zig`, OSC 99 keys `p`/`d`/`e`/`i` and chunking in `kitty_notification.zig` (2048-byte fields). The pane tab's unread dot came from M2's research (Bonsplit, 6 px).
- Findings that shaped the plan: notifications are created unread even on the focused pane (a focused-read indicator keeps the ring until you type); at most one per surface, no count cap; BEL never notifies; the flash is a 0.9 s double blink separate from the static ring; banners show unless the arrival is the exact focused surface of the active app.
- Decisions: D-Bus via `zbus`, already in `Cargo.lock` through gpui's ashpd (no new packages, libc unmoved); a click activates the window with the notification's `xdg_activation` token through a shim like `blur.rs`, because gpui 0.2.2's own `activate` asks for a token KWin refuses for a background app.
- Deviations planned: the panel is an overlay under a bell in the sidebar band (no titlebar), no resize/saved size/context menu, 24 h clock; BEL, manual unread, settings and the taskbar badge deferred (plan Deferred list).
