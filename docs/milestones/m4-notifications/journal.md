# m4-notifications — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-10 — Research and plan

- Branch `m4-notifications` from `master`. The M2/M3 desktop exit checks are still pending; the owner chose to start M4 meanwhile (this session ran on the macOS host).
- Research: `research-cmux.md` (cmux @ 21dcc2b, cited `rn §n`). Its two UNVERIFIED points were settled from the vendored Ghostty fork (MIT): OSC 9 ConEmu rules in `osc/parsers/osc9.zig`, OSC 99 keys `p`/`d`/`e`/`i` and chunking in `kitty_notification.zig` (2048-byte fields). The pane tab's unread dot came from M2's research (Bonsplit, 6 px).
- Findings that shaped the plan: notifications are created unread even on the focused pane (a focused-read indicator keeps the ring until you type); at most one per surface, no count cap; BEL never notifies; the flash is a 0.9 s double blink separate from the static ring; banners show unless the arrival is the exact focused surface of the active app.
- Decisions: D-Bus via `zbus`, already in `Cargo.lock` through gpui's ashpd (no new packages, libc unmoved); a click activates the window with the notification's `xdg_activation` token through a shim like `blur.rs`, because gpui 0.2.2's own `activate` asks for a token KWin refuses for a background app.
- Deviations planned: the panel is an overlay under a bell in the sidebar band (no titlebar), no resize/saved size/context menu, 24 h clock; BEL, manual unread, settings and the taskbar badge deferred (plan Deferred list).

## 2026-11-10 — Tasks 1–8 built (inline, on the macOS host); desktop checks pending

- Built task by task in one session (plan "native" execution), TDD for every pure piece; Linux CI compiled the Linux-only D-Bus and Wayland code (Tasks 7–8).
- macOS host, not M4: `cmux-term` test `terminal_replies_are_written_back_to_the_child` fails and clippy flags an unused `ClipboardItem` import in `mouse.rs` (used only on Linux). Both predate M4; Linux CI is the gate.
- Rulings (details in the plan's execution ledger, summarised here):
  - OSC 99: an overflowing chunk poisons that id until it completes (dropped), where Ghostty keeps appending. A small base64 decoder instead of a new `cmux-term` dependency. An empty OSC 9 stays dropped.
  - The ring and flash glow is a wider faint stroke, not a gpui shadow: gpui 0.2.2 paints a shadow as a filled blurred rect, which would tint the terminal.
  - `cmux-app` gains `libc` (local clock) and `zbus` (Linux); `Cargo.lock` changed only in `cmux-app`'s dependency list, libc still 0.2.189.
  - The panel's "Jump to Latest" closes the panel; no shortcut chip. A 300 ms guard keeps the bell from reopening a panel its own click just closed.
  - `blur.rs`'s guest `wl_display`/`wl_surface` setup moved to `wayland.rs`, shared with the new `activation.rs`.
- Deviations from cmux: as in the plan (D7 panel under a bell in the sidebar band, no resize/saved size/context menu, 24 h clock; Deferred list: BEL, manual unread, settings, taskbar badge, macOS bubbles).
- Whole-branch review (one fresh reviewer) found that a new notification on a surface closed the very bubble it reused (`replaces_id`), so every other KDE bubble vanished. Fixed with one post in flight per surface (newest queued, replaced bubbles kept). Also fixed: token/click pairing now reads one ordered D-Bus stream; a jump makes the target tab active before switching workspace. Deferred minors: per-post error logging, the panel's time-based reopen guard, base64 padding laxer than Ghostty's.
- Pending at the desktop (plan Task 9 Step 1): every "Run it" step of Tasks 4–6 (screenshots `ring.png`, `tab-dot.png`, `sidebar.png`, `panel.png`, `panel-empty.png` to `~/Pictures/cmux-m4/`), exits 1–3, OSC 777/99 and Claude Code's hook, and a blur re-check (the shared `wayland.rs`).
