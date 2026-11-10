# M4 notifications Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An agent waiting for input is impossible to miss, as in cmux. OSC 9/99/777 creates a notification. The pane gets a ring and flash, its tab a dot, and its workspace row the message and an unread badge. The panel (Super+I) lists notifications, Super+Shift+U jumps to the latest unread, and a KDE desktop notification focuses the pane when clicked. Read/unread rules follow cmux.

**Architecture:**
- `cmux-term` turns OSC 9 (with Ghostty's ConEmu rules), OSC 99 (kitty, chunked) and OSC 777 into `TermEvent::Notify { title, body }` and drops machine payloads.
- `cmux-core` gets `Notifications`: a pure store, newest first, at most one per (workspace, surface), with cmux's read rules as methods. It is unit-tested without a window.
- `cmux-app`: `WindowView` owns the store and gets `Notify`/`Focused`/`Typed` events from each `WorkspaceView`. It pushes ring state into the selected workspace and draws the sidebar badge, the message line, the bell and the panel overlay. On Linux it posts freedesktop notifications through `zbus`, and when one is clicked it activates the window with the notification's `xdg_activation` token through a small shim like `blur.rs`.

**Tech Stack:**
- Rust 2024 (toolchain 1.99.0), gpui `=0.2.2`, libghostty-vt (vendored), libc 0.2.
- Linux only: zbus 5.19.0 (`default-features = false, features = ["async-io"]`; already in `Cargo.lock` through gpui's ashpd), wayland-client / wayland-protocols (already used by `blur.rs`).
- Lucide icons `bell.svg`, `bell-off.svg` (ISC, as the existing icons).

**Spec:** `docs/milestones/m4-notifications/spec.md`. Research: `research-cmux.md` (cited `rn §n`) in the same folder. Ghostty's own OSC parsers are in `vendor/ghostty/src/terminal/osc/parsers/` (MIT; `osc9.zig`, `kitty_notification.zig`). The pane tab's unread dot is in M2's `research-cmux.md` (Bonsplit `TabItemView.swift:1159-1172`).

## Global Constraints

Carried from M0–M3. Every task's requirements include these:
- Licence: GPL-3.0-or-later for our crates. Every added dependency, vendored file or asset goes in `NOTICE` with its licence. Never copy from cmux's BUSL-1.1 directories.
- Zig must be exactly 0.16.0 (`scripts/install-zig.sh`). Run `export PATH="$PWD/.tools/zig:$PATH"` from the repo root before any cargo command.
- Pass `--locked` to build, test and clippy. The one exception is Task 7's dependency step, which is allowed to change `Cargo.lock`, and then only the `cmux-app` dependency list.
- Keep libc pinned at 0.2.189 in `Cargo.lock`. Never run a bare `cargo update`. Check with `grep -A1 'name = "libc"' Cargo.lock`.
- Keep gpui pinned at `=0.2.2` and the toolchain at `rust-toolchain.toml`.
- No personal info in tracked files.
- `STATUS.md` is max 100 lines. Append it to `docs/status-archive.md` before replacing it.
- Before each commit, run `cargo fmt -p <crates you touched>` and `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`.
- Dependency rules: `cmux-core` depends on nothing; `cmux-term` and `cmux-config` never depend on gpui. Linux-only code (D-Bus, Wayland) sits behind `cfg(target_os = "linux")` and degrades to "nothing happens" elsewhere.
- UI sizes are cmux points used as logical pixels (1 pt = 1 px). Accent = `chrome::cmux_accent` (#0088FF light / #0091FF dark, rn §4).
- macOS host builds use `--features gpui/runtime_shaders`. `cfg(linux)` code (Tasks 7–8) is only compiled by Linux CI: push and check the run (`gh run watch`) before ticking those tasks.

New for M4:
- Branch `m4-notifications` from `master`. It merges at Task 9.
- **One commit per task step that changes files.** Messages end with a blank line and the session's attribution lines.
- As in M3, this plan's code was not compiled. Signatures are exact, but gpui call details may need adjusting. Names a later task consumes must match the **Interfaces** blocks, and any change goes in the journal.
- If behaviour is unclear, read the cmux file rn §n cites and match it. Record deviations in the journal.
- Screenshots go to `~/Pictures/cmux-m4/`, outside the repo.

## Review Focus

1. **A program spamming notifications** (`while true; printf '\e]9;x\a'; done`). The store stays at one entry per surface. KDE gets one replacing bubble per surface, not hundreds of new ones (`replaces_id`), and the UI only redraws when the store changes. Tests: Task 2 `add_replaces_same_surface`, Task 7 `replaces_id_follows_the_surface`.
2. **Hostile or odd payloads.** Markup (`<b>`, `&`) in a body sent to KDE, C0 controls, URLs as titles, JSON bodies, bad base64, and an OSC 99 chunk that never says `d=1`. The result: escaped text, a dropped payload, or a bounded buffer. Never a panic or growing memory. Tests: Task 1 `machine_payloads_are_dropped`, `kitty_unfinished_chunks_are_bounded`; Task 7 `body_is_escaped_only_with_markup`.
3. **Clicking a stale desktop notification** whose surface or workspace has since closed. Nothing happens, and it must not panic or focus a wrong pane. Test: Task 2 `ids_of_closed_surfaces_are_gone`; Task 7 `unknown_server_id_maps_to_nothing`.
4. **No notification daemon or no session bus** (CI, a bare WM, an SSH session). The app runs, logs one line, and everything except desktop bubbles still works. Test: Task 7 `connect_without_bus_is_none` (Linux).
5. **A notification for the focused pane while the window is in the background.** That is not a focused arrival. It must still post a desktop notification and flash. Test: Task 4 `inactive_window_is_never_a_focused_arrival`.

## Decisions made in this plan

| Decision | Why |
|---|---|
| **D1 Parsing.** `TermEvent::Notify { title: String, body: String }` (an empty title is allowed). OSC 9: the ConEmu forms `9;1`–`9;12` that Ghostty's `osc9.zig` accepts are **not** notifications; any form it rejects falls through as a body. OSC 777: `notify;title;body`. OSC 99: metadata keys `p` (`title` by default, `body`, or `close`/`alive` = ignore), `d` (done, default 1), `e` (base64), `i` (id, `[A-Za-z0-9-_+.:]`). Chunks with the same id accumulate until `d=1`; a chunk without an id resets. Each field is capped at 2048 bytes (the whole notification is dropped if over). An empty title with a body moves the body to the title. | rn §1 says parsing happens in Ghostty. Ghostty's parsers are vendored and MIT, so we match them exactly. libghostty-vt does not report them, so our existing `OscScanner` stays. |
| Machine payloads are dropped: a title that starts with `scheme://`, any C0 control except `\n \r \t` or 0x7f–0x9f in the title or body, and a body that parses as a JSON object or array. | rn §1 (`HookResolution`). The JSON check is a cheap "starts with `{`/`[` and ends with `}`/`]` after trim" test. `shortcut:` comment; use serde_json if false positives appear. |
| **D2 Store.** `cmux_core::Notifications`, newest first. Adding a notification removes every existing one with the same (workspace, surface), read or not. There is no count cap. Notifications are keyed by `(WorkspaceId, SurfaceId)` because SurfaceIds restart per workspace. | rn §2. With one per surface the size is bounded by open surfaces. |
| **D3 Read rules.** A notification is created **unread even when focused**. A focused arrival also sets a *focused-read indicator* for that surface. It is marked read when its surface gains focus with the window active, when its workspace is selected with the window active (only that workspace's active surface), when the window becomes active (the selected workspace's active surface), on a jump or panel click, or by mark-all-read. **Typing** into a surface marks it read and clears the indicator, even with the window inactive. Focus does not clear the indicator. Closing a surface or workspace deletes its notifications. | rn §3. The ring therefore stays on a pane that was focused when the notification arrived, until you type. |
| Focused arrival = window active **and** workspace selected **and** surface is that workspace's active surface. A focused arrival posts no desktop notification, does not flash, and does not reorder. | rn §3, §9 (`suppressWhenAppFocused` false). |
| A non-focused arrival moves its workspace to the **top** of the list. | rn §6 `app.reorderOnNotification` default. There are no pinned tiers yet. |
| **D4 Ring.** Shown while `shows_ring` (unread on that surface, or the indicator). It is a rounded rect inset 2 from the pane's content area, radius 6, 2 px accent stroke, with glow (shadow at 0.35 alpha, blur 3). It also shows on the focused pane and has no animation. The pane is the area below the tab strip, the terminal only. | rn §4. |
| **Flash:** the same rect, glow 0.6/6, a double blink lasting 0.9 s with opacity keyframes 0→1→0→1→0 at 0, .25, .5, .75 and 1, quadratic ease-out going up and ease-in coming down. It runs on a non-focused arrival in the selected workspace, and when a surface's unread goes read through focus, typing, jump or click. A new flash is ignored while one is still running on that surface. | rn §4. `paneFlashOnTyping` defaults to true. |
| **D5 Tab dot.** A 6 px accent circle in the close-button slot on a tab that is neither selected nor hovered while `shows_ring` holds. | M2 rc (Bonsplit `TabItemView.swift:1159-1172`), rn §5. |
| **D6 Sidebar.** Below the title line comes the latest notification of the workspace (read or unread): its body, else its title, trimmed, at 10 px regular in the detail colour × 0.8, wrapping up to **12** lines, tail-truncated, and capped at 4096 chars before layout. The unread badge is a 16 px circle in the **leading** slot of the title line, showing the count at 9 px semibold, white on accent. On the selected row it is filled with the title colour at 0.25 and the text uses the title colour. It is shown only when the count is above 0, with no cap. | rn §6. |
| **D7 Panel.** cmux's titlebar popover becomes an overlay anchored under a **bell button** added to the sidebar band, after the toggle (M3 already put the titlebar buttons in that band). The bell carries an 11–13 px accent badge showing `min(count, 99)`. The panel is 560×760, clamped to the window minus 16 px. Header: "Notifications" (14 semibold), an unread capsule, "Jump to Latest" (disabled when nothing is unread) and "Clear All" (disabled when empty). Rows are at least 56 px high, newest first: a 2.5 px accent bar when unread; the workspace title (12.5 semibold) with the time (10.5, `HH:MM` local) on the right; the notification title (10.5 medium); the body (11.5, 2 lines). Hovering tints the row (0.11) and shows a 20 px ×. Clicking a row jumps and closes the panel; × removes that notification. Empty state: `bell-off`, "No notifications yet", "Desktop notifications will appear here." Escape or a click outside closes the panel. Super+I toggles it, and when the sidebar is hidden it anchors at the window's top-left. | rn §7. Deviations: no resize handle or saved size, no context menu, and a 24 h clock (std has no locale). |
| **D8 Jump to latest unread** (Super+Shift+U): the newest unread notification across workspaces. It selects that workspace, focuses the surface (selecting its tab), marks the notification read and flashes. With none unread it does nothing. | rn §8. There are no manual-unread flags yet (see Deferred). |
| **D9 Desktop (Linux).** `org.freedesktop.Notifications.Notify` over the session bus via zbus: app name `cmux-linux`, `replaces_id` = the server id of the previous bubble for the same (workspace, surface), summary = title, body (escaped `& < >` only when `GetCapabilities` lists `body-markup`), actions `["default", "Open"]`, hints `desktop-entry=cmux-linux`, `sound-name=message-new-instant`, timeout −1. Not posted for focused arrivals. A bubble is withdrawn (`CloseNotification`) when its notification is read or removed. On `ActivationToken` the token is kept; on `ActionInvoked(id, "default")` the window is activated (Wayland: `xdg_activation_v1.activate(token, surface)`; otherwise `window.activate_window()`) and the jump of D8 runs for that notification. No bus or no daemon means one stderr line and no desktop notifications. | rn §9. `zbus` is already in the tree through gpui, so libc doesn't move. gpui 0.2.2's own `activate` asks the compositor for a fresh token, which KWin refuses for a background app. The notification's token is accepted. |
| An empty title becomes the workspace title (custom, else OSC, else "Terminal") when the notification is stored. | rn §1. |

## Deferred (with reason)

- BEL and Ghostty `bell-features` (beep, visual bell, manual unread). The spec names only OSC 9/99/777. M6 parity.
- Manual unread flags: `toggleUnread`, `markOldestUnreadAndJumpNext`, "Mark Workspace as Read/Unread", workspace mute. They need a second unread source. M6 or later.
- Settings (`unreadPaneRing`, `paneFlash*`, `sound`, `notifications.command`, `suppress*`, sidebar notification options): the cmux defaults are hard-wired. Add with a settings file.
- `triggerFlash`, the panel's context menu, resize and saved size, a taskbar badge (Unity `LauncherEntry`), macOS desktop notifications: M6.
- `cmux notify` and workspace-level notifications: M5 (the store keys stay ready for it).

## File structure

| File | Change |
|---|---|
| `crates/cmux-term/src/osc.rs` | OSC 9 ConEmu rules, OSC 99, machine-payload filter, `Notify { title: String, body: String }` |
| `crates/cmux-term/src/lib.rs` | `TermEvent::Notify` field types follow |
| `crates/cmux-core/src/notifications.rs` (new), `lib.rs` | the store |
| `crates/cmux-config/src/keybindings.rs` | four new actions |
| `crates/cmux-app/src/actions.rs` | four new gpui actions |
| `crates/cmux-app/src/terminal_view.rs` | emits `Notify`, `Typed` |
| `crates/cmux-app/src/workspace.rs`, `workspace/tab_strip.rs` | emits `Notify`/`Focused`/`Typed`; ring, flash, tab dot, `focus_surface` |
| `crates/cmux-app/src/attention.rs` (new) | pure: `flash_opacity`, `is_focused_arrival`, ring/flash elements |
| `crates/cmux-app/src/window_view.rs`, `window_view/notify.rs` (new) | store ownership, rules wiring, jump, actions |
| `crates/cmux-app/src/window_view/sidebar.rs` | badge, message line, bell button |
| `crates/cmux-app/src/window_view/panel.rs` (new) | notification panel overlay |
| `crates/cmux-app/src/desktop_notify.rs` (new, Linux) | zbus client, `DesktopIds`, `body_text` |
| `crates/cmux-app/src/activation.rs` (new, Linux) | `xdg_activation_v1` shim |
| `crates/cmux-app/assets/icons/bell.svg`, `bell-off.svg` | Lucide |
| `README.md`, `NOTICE`, `docs/design.md`, `STATUS.md`, journal | Task 9 |

---

### Task 1: cmux-term: OSC 9 / 99 / 777 as Ghostty parses them

**Files:** Modify `crates/cmux-term/src/osc.rs`, `crates/cmux-term/src/lib.rs`, `crates/cmux-app/src/terminal_view.rs` (only the match arm, so the workspace compiles).

**Interfaces:**
- Produces: `OscEvent::Notify { title: String, body: String }` and `TermEvent::Notify { title: String, body: String }`. For OSC 9, `title` is `""`.
- `OscScanner` keeps its `new()` / `feed(&[u8]) -> Vec<OscEvent>`. It now also holds the OSC 99 pending state (`id: Option<String>`, `title: String`, `body: String`).

- [ ] **Step 1: Write the failing tests** in `osc.rs`'s test module:
  - `osc9_conemu_table`: one case for each `test "OSC 9…"` in `vendor/ghostty/src/terminal/osc/parsers/osc9.zig`, with input and expectation copied. ConEmu forms give `vec![]`; the "-> desktop notification" cases give `Notify { title: "", body: <their body> }`. Example rows: `"9;4;1;50"` → none; `"9;1;abc"` → whatever the zig test expects (read it, don't guess).
  - `osc99_title_then_body_chunks`: `\e]99;i=a:d=0;Hi\a` then `\e]99;i=a:p=body;there\a` → one `Notify { title: "Hi", body: "there" }`, emitted only after the second.
  - `osc99_body_only_becomes_title`: `\e]99;p=body;x\a` → `Notify { title: "x", body: "" }`.
  - `osc99_base64`: `\e]99;e=1;SGk=\a` → title `"Hi"`; bad base64 → none.
  - `osc99_close_and_alive_are_ignored`, `osc99_new_id_resets_pending`.
  - `kitty_unfinished_chunks_are_bounded`: 2048 bytes of title over `d=0` chunks with one id → none, and a following `\e]99;;ok\a` → `Notify { title: "ok", .. }`.
  - `machine_payloads_are_dropped`: `777;notify;https://x;b`, `9;a\x01b` (raw 0x01 inside the payload), `9;{"a":1}`, `9;[1]` → none. `9;line1\nline2` → kept.
  - Update the existing `Notify` tests to the new field types.
- [ ] **Step 2: Run them to see them fail:** `cargo test -p cmux-term --locked osc::` fails (compile errors or wrong values).
- [ ] **Step 3: Implement** in `osc.rs`. `parse` becomes a `&mut self` method because OSC 99 is stateful. Port the ConEmu acceptance rules from `osc9.zig` (which subcommands and argument shapes it accepts), not its effects. Add `fn is_machine_payload(title: &str, body: &str) -> bool`. Remove the M1 `shortcut:` comment about ConEmu. Change `lib.rs`'s `TermEvent::Notify` and `terminal_view.rs`'s arm (still ignored there until Task 4).
- [ ] **Step 4: Run** `cargo test -p cmux-term --locked` and expect all to pass. Then run clippy.
- [ ] **Step 5: Commit** `cmux-term: OSC 9 ConEmu rules, OSC 99, machine payload filter`.

### Task 2: cmux-core: the notification store

**Files:** Create `crates/cmux-core/src/notifications.rs`; modify `crates/cmux-core/src/lib.rs` (`mod notifications; pub use notifications::*;`).

**Interfaces (Produces):**
```rust
pub struct NotificationId(pub u64);              // Copy, Eq, Hash, Debug
pub struct Notification {
    pub id: NotificationId, pub workspace: WorkspaceId, pub surface: SurfaceId,
    pub title: String, pub body: String, pub created: std::time::SystemTime, pub read: bool,
}
impl Notification { pub fn summary(&self) -> Option<&str> }  // trimmed body, else trimmed title, None if both empty
pub struct Added { pub id: NotificationId, pub replaced: Vec<NotificationId> }
pub struct Notifications { /* newest-first Vec, focused-read indicator set, next id, generation */ }
impl Notifications {
    pub fn new() -> Self;
    pub fn generation(&self) -> u64;                // bumps on every change, never otherwise
    pub fn add(&mut self, ws: WorkspaceId, s: SurfaceId, title: String, body: String,
               focused: bool, now: SystemTime) -> Added;
    pub fn list(&self) -> &[Notification];          // newest first
    pub fn get(&self, id: NotificationId) -> Option<&Notification>;
    pub fn unread_count(&self, ws: WorkspaceId) -> usize;
    pub fn total_unread(&self) -> usize;
    pub fn latest(&self, ws: WorkspaceId) -> Option<&Notification>;      // read or not
    pub fn latest_unread(&self) -> Option<&Notification>;
    pub fn shows_ring(&self, ws: WorkspaceId, s: SurfaceId) -> bool;     // unread there, or indicator
    // Each returns the ids that became read or were removed (for desktop withdrawal):
    pub fn mark_read(&mut self, id: NotificationId) -> Vec<NotificationId>;
    pub fn mark_surface_read(&mut self, ws: WorkspaceId, s: SurfaceId) -> Vec<NotificationId>; // keeps indicator
    pub fn typed(&mut self, ws: WorkspaceId, s: SurfaceId) -> Vec<NotificationId>;            // read + clear indicator
    pub fn mark_all_read(&mut self) -> Vec<NotificationId>;
    pub fn remove(&mut self, id: NotificationId) -> Vec<NotificationId>;
    pub fn clear_all(&mut self) -> Vec<NotificationId>;
    pub fn remove_surface(&mut self, ws: WorkspaceId, s: SurfaceId) -> Vec<NotificationId>;
    pub fn remove_workspace(&mut self, ws: WorkspaceId) -> Vec<NotificationId>;
}
```
`remove*` and `clear_all` also drop the matching indicators. `typed` on a surface that has nothing to change leaves `generation` unchanged: it is called on every keystroke.

- [ ] **Step 1: Write the failing tests:**
  - `add_is_unread_and_newest_first`
  - `add_replaces_same_surface`: 1000 adds on one surface → `list().len() == 1`, and each `Added.replaced` holds the previous id.
  - `same_surface_id_in_two_workspaces_is_distinct`
  - `focused_arrival_is_unread_with_indicator`: `add(.., focused: true, ..)` gives read = false and `shows_ring` true. After `mark_surface_read`, `shows_ring` is still true. After `typed`, it is false.
  - `unfocused_arrival_ring_ends_on_read`
  - `typed_without_change_keeps_generation`
  - `latest_includes_read`, `summary_prefers_body_then_title`
  - `latest_unread_spans_workspaces`
  - `counts`
  - `ids_of_closed_surfaces_are_gone`: after `remove_surface`/`remove_workspace`, `get` and `mark_read` on the old id return `None`/empty, and the indicator is gone.
  - `mark_all_read_and_clear_all_return_affected_ids`
- [ ] **Step 2: Run** `cargo test -p cmux-core --locked notifications::` and expect it to fail.
- [ ] **Step 3: Implement** to the interface above.
- [ ] **Step 4: Run** `cargo test -p cmux-core --locked` and expect it to pass.
- [ ] **Step 5: Commit** `cmux-core: notification store with cmux's read rules`.

### Task 3: Keybindings and actions

**Files:** Modify `crates/cmux-config/src/keybindings.rs` (default table and its tests), `crates/cmux-app/src/actions.rs`.

**Interfaces (Produces):** gpui actions `ShowNotifications`, `JumpToUnread`, `MarkAllNotificationsRead`, `ClearAllNotifications`, mapped from the config names `showNotifications` (`super+i`), `jumpToUnread` (`super+shift+u`), `markAllNotificationsRead` (unbound), `clearAllNotifications` (unbound) (rn §10).

- [ ] **Step 1: Write the failing tests.** Extend the keybindings default-table test with the four rows above, and extend the existing actions sync test (every config action has a gpui action) so it covers them.
- [ ] **Step 2: Run** `cargo test -p cmux-config --locked` and `cargo test -p cmux-app --locked actions` and expect them to fail.
- [ ] **Step 3: Implement.** Add the table rows and the `actions!` and `action_for` entries. The handlers come in Task 4.
- [ ] **Step 4: Run** the tests and expect them to pass.
- [ ] **Step 5: Commit** `notification shortcuts: Super+I, Super+Shift+U, mark all read, clear all`.

### Task 4: Wiring: events, read rules, reorder, jump

**Files:**
- Modify `terminal_view.rs`, `workspace.rs`, `window_view.rs`.
- Create `window_view/notify.rs` (an `impl WindowView` block, like `sidebar.rs`) and `attention.rs` (only `is_focused_arrival` in this task).

**Interfaces:**
- Consumes: Task 1 `TermEvent::Notify`, Task 2 `Notifications`, Task 3 actions.
- Produces:
  - `TerminalEvent::Notify { title: String, body: String }` and `TerminalEvent::Typed`. `Typed` is emitted when keyboard input or a paste writes to the PTY. Mouse reports don't count.
  - `WorkspaceEvent::Notify { surface: SurfaceId, title: String, body: String }`, `WorkspaceEvent::Focused(SurfaceId)` (from the existing `on_focus_in`, every time), `WorkspaceEvent::Typed(SurfaceId)` and `WorkspaceEvent::Closed(SurfaceId)`.
  - `WorkspaceView::focus_surface(&mut self, s: SurfaceId, window, cx) -> bool`: selects the tab and focuses it.
  - `WorkspaceView::set_attention(&mut self, ring: HashSet<SurfaceId>)`.
  - `WorkspaceView::flash(&mut self, s: SurfaceId, cx)`: only stores a request in this task; Task 5 draws it.
  - `pub fn is_focused_arrival(window_active: bool, workspace_selected: bool, surface_is_active: bool) -> bool` in `attention.rs`.
  - `WindowView` fields `notifications: Notifications`, `panel_open: bool`, and `fn jump_to(&mut self, id: NotificationId, window, cx) -> bool`.
  - `fn after_change(&mut self, gone: Vec<NotificationId>, cx)`: re-pushes the selected workspace's ring set and notifies. Task 7 also withdraws `gone` there.

- [ ] **Step 1: Write the failing test** `inactive_window_is_never_a_focused_arrival` in `attention.rs`. It covers all 8 combinations; only `(true, true, true)` is a focused arrival.
- [ ] **Step 2: Run** `cargo test -p cmux-app --locked attention` and expect it to fail. Then implement `is_focused_arrival` and run again to see it pass. Commit `attention: focused-arrival rule`.
- [ ] **Step 3: Implement the wiring** (D3, D8):
  - `on_pty_events` emits `Notify`. The key and paste paths emit `Typed`.
  - `WorkspaceView` forwards all three with its `SurfaceId`.
  - In `WindowView::make_view`'s subscription:
    - **Notify** fills an empty title (last row of the D table) and computes the focused arrival with `window.is_window_active()`. It calls `add`. A non-focused arrival moves the workspace to index 0 with `Workspaces::move_to(id, 0)`, and when the workspace is selected it flashes the surface. Then `after_change`.
    - **Focused(s)**: when the window is active and this is the selected workspace, `mark_surface_read`. If anything became read, flash.
    - **Typed(s)**: `typed`. If anything became read, flash.
  - In `select()`: when the window is active, `mark_surface_read` the new workspace's active surface, then `set_attention`. Also add the one-line M3 carry-over: `set_left_inset` in `select`.
  - `cx.observe_window_activation` in `WindowView::new`: on becoming active, mark the selected workspace's active surface read.
  - `WorkspaceView` emits a new `WorkspaceEvent::Closed(SurfaceId)` for every surface it drops: in `close_surface`, before `Empty` on the last one, and for each id in `close_other_tabs`. That event calls `remove_surface`, and `close_now` calls `remove_workspace`.
  - Handlers: `JumpToUnread` runs `latest_unread` then `jump_to`; `MarkAllNotificationsRead`; `ClearAllNotifications`; `ShowNotifications` toggles `panel_open` (drawn in Task 6).
  - `jump_to`: if the workspace or surface no longer exists, return false. Otherwise select the workspace, `focus_surface`, `mark_read`, flash.
- [ ] **Step 4: Run it.** On the macOS dev build (or Linux), check that `printf '\e]9;hi\a'` in an unfocused pane moves its workspace to the top. With a `dbg!` or `eprintln!` of the store (remove it before committing), check that typing in the pane marks it read and Super+Shift+U lands on the right pane. Run the full test suite and clippy.
- [ ] **Step 5: Commit** `notifications: events, read rules, reorder, jump to latest unread`.

### Task 5: Pane ring, flash and tab dot

**Files:** Modify `attention.rs`, `workspace.rs` (`render_pane`), `workspace/tab_strip.rs`, `chrome.rs` (only if a colour helper is missing).

**Interfaces:**
- Consumes: Task 4 `set_attention` and `flash`.
- Produces:
  - `pub fn flash_opacity(t: f32) -> f32` for t in [0, 1].
  - `pub fn ring(accent: Rgba) -> Div` and `pub fn flash(accent: Rgba, token: u64) -> impl IntoElement`: absolute overlays, inset 2, radius 6, 2 px border. Ring glow is 0.35/3; flash glow is 0.6/6 under `with_animation(("flash", token), Animation::new(900ms), |el, t| el.opacity(flash_opacity(t)))`.
  - `WorkspaceView` keeps `flashes: HashMap<SurfaceId, (u64, Instant)>`. `flash()` ignores a request less than 900 ms after the last one for that surface, bumps the token, and schedules one `cx.notify` after 900 ms to drop the overlay.

- [ ] **Step 1: Write the failing test** `flash_opacity_double_blink`. Expected values: t = 0 → 0, 0.25 → 1, 0.5 → 0, 0.75 → 1, 1 → 0. Inside the segments: 0.125 → 0.75 (ease-out quadratic going up) and 0.375 → 0.75 (ease-in quadratic coming down). Allow a tolerance of 1e-4.
- [ ] **Step 2: Run** it, expect it to fail, then implement it and run again to see it pass.
- [ ] **Step 3: Draw the overlays.** In `render_pane`, the ring and flash go over the terminal area (beside the dim overlay) and take no mouse handlers. In `render_tab`, a tab that is neither selected nor hovered and whose surface is in the ring set shows a 6 px accent dot in place of the close glyph (D5).
- [ ] **Step 4: Run it.** Put two panes in one workspace. A notification in the unfocused pane should double-blink, then leave a steady ring. A notification in the focused pane should give a ring with no blink, and the ring should go when you type there. In a pane with two tabs, a notification on the hidden tab should show the dot. Take screenshots `ring.png`, `tab-dot.png`. Run tests and clippy.
- [ ] **Step 5: Commit** `pane ring, double-blink flash and tab unread dot`.

### Task 6: Sidebar badge, message line, bell and the notification panel

**Files:**
- Modify `window_view/sidebar.rs` (`render_row`, band).
- Create `window_view/panel.rs` and `assets/icons/bell.svg`, `bell-off.svg` (Lucide, unmodified).
- Modify `assets.rs` if icons are listed there, and `Cargo.toml` (`libc.workspace = true` for `cmux-app`; libc is already in the lock).

**Interfaces:**
- Consumes: the store queries from Task 2, and `jump_to` / `panel_open` from Task 4.
- Produces:
  - `pub(crate) fn badge_label(count: usize) -> String` (the sidebar shows the full count) and `pub(crate) fn bell_label(count: usize) -> Option<String>` (`None` at 0, else `min(count, 99)`).
  - `pub(crate) fn clip_message(s: &str) -> &str` (at most 4096 chars, on a char boundary).
  - `pub(crate) fn clock(t: SystemTime) -> String` (`HH:MM` local, using `libc::localtime_r`).
  - `WindowView::render_panel(&self, window, cx) -> Option<impl IntoElement>`.

- [ ] **Step 1: Write the failing tests:**
  - `bell_label_caps_at_99`: 0 → None, 7 → "7", 150 → "99".
  - `badge_label_has_no_cap`: 150 → "150".
  - `clip_message_respects_char_boundaries`: 5000 "é" → 4096 chars.
  - `clock_is_two_digit_24h`: matches `^\d\d:\d\d$`.
- [ ] **Step 2: Run** them, expect them to fail, then implement and run again to see them pass.
- [ ] **Step 3: Sidebar (D6).** Add the badge in the title line's leading slot and the message block under the title line, using `line_clamp(12)` and `text_ellipsis`. Add a bell button to the band after the toggle, with its badge offset (+3, −3) from its top-right; clicking it toggles `panel_open`.
- [ ] **Step 4: Panel (D7).** Build it as `deferred(anchored().position(<under the bell>).child(..))` with `on_mouse_down_out` and Escape to close. Implement the header, rows, hover ×, empty state, "Jump to Latest" and "Clear All" exactly as D7 lists them. Every row, button and badge reads from `self.notifications` only: never from a hidden `WorkspaceView`, so hidden views are not tracked (see M3's `titles` cache). A workspace title comes from `title_of`.
- [ ] **Step 5: Run it.** Use 5 workspaces with notifications, some read. Check that the badges count unread only, that the message is still shown after reading, and that the panel lists newest first. Clicking a row should jump and close; × should remove; "Clear All" should empty it and show the empty state; Super+I should toggle. Take screenshots `sidebar.png`, `panel.png`, `panel-empty.png`. Run tests and clippy.
- [ ] **Step 6: Commit** `sidebar notification badge and message, bell, notification panel`.

### Task 7: Linux desktop notifications over D-Bus

**Files:**
- Create `crates/cmux-app/src/desktop_notify.rs` (`#[cfg(target_os = "linux")]`).
- Modify `crates/cmux-app/Cargo.toml` (Linux target: `zbus = { version = "5.19", default-features = false, features = ["async-io"] }`), `Cargo.lock`, `window_view/notify.rs`, `lib.rs`.

**Interfaces:**
- Consumes: Task 4 `after_change` and `jump_to`.
- Produces:
  - `pub struct DesktopIds`: a two-way `NotificationId` ↔ server `u32` map, plus the last server id for each `(WorkspaceId, SurfaceId)`. It has:
    - `fn replaces(&self, ws, s) -> u32` (0 = none);
    - `fn record(&mut self, id, ws, s, server: u32)`;
    - `fn server_of(&self, id) -> Option<u32>`;
    - `fn local_of(&self, server: u32) -> Option<NotificationId>`;
    - `fn forget(&mut self, id) -> Option<u32>`.
  - `pub fn body_text(body: &str, markup: bool) -> String`.
  - `pub struct DesktopNotifier { conn: zbus::Connection, markup: bool }` with:
    - `async fn connect(address: Option<&str>) -> Option<Self>`: the session bus when `None`, else that address via `zbus::connection::Builder::address`; then one `GetCapabilities` call. On error it logs `cmux-linux: desktop notifications off: <e>` once.
    - `async fn post(&self, summary: &str, body: &str, replaces: u32) -> Option<u32>`.
    - `async fn close(&self, server: u32)`.
    - `fn signals(&self) -> impl Stream<Item = Signal>`, with `pub enum Signal { Activated { server: u32, token: Option<String> }, Closed(u32) }`, which merges `ActivationToken` and `ActionInvoked("default")` by id.
  - `WindowView` fields under the Linux cfg: `desktop: Option<Rc<DesktopNotifier>>`, `desktop_ids: DesktopIds`.

- [ ] **Step 1: Add the dependency.** Run `cargo build -p cmux-app` once **without** `--locked`. Then `git diff Cargo.lock` must show only `"zbus"` added to `cmux-app`'s dependency list, no version changes and no new packages, and libc must still be 0.2.189. If anything else moves, stop and record it in the journal. Add zbus (MIT) to `NOTICE`. Commit `cmux-app: zbus for desktop notifications (already in the tree)`.
- [ ] **Step 2: Write the failing tests** (pure, so they run on any OS; gate only the zbus parts):
  - `replaces_id_follows_the_surface`
  - `unknown_server_id_maps_to_nothing`
  - `forget_clears_both_directions`
  - `body_is_escaped_only_with_markup`: `a<b>&c` → `a&lt;b&gt;&amp;c` with markup, and unchanged without it.
  - `connect_without_bus_is_none` (Linux): `connect(Some("unix:path=/nonexistent"))` returns `None` without a panic, and the process env is left untouched.
- [ ] **Step 3: Run** `cargo test -p cmux-app --locked desktop_notify` and expect it to fail. Then implement and run again to see it pass. Commit `desktop_notify: id map, escaping, zbus client`.
- [ ] **Step 4: Wire it (D9).**
  - Connect on the background executor in `WindowView::new`.
  - On a non-focused `add`, `post` with `replaces(ws, s)`, then `record`.
  - In `after_change`, `forget` and `close` every `gone` id.
  - A foreground task reads `signals()`:
    - `Activated`: `local_of` the server id. Wayland with a token: `activation::activate(window, token)` (Task 8); otherwise `window.activate_window()`. Then `jump_to`.
    - `Closed`: `forget` the local id.
  - Push and confirm Linux CI is green.
- [ ] **Step 5: Commit** `post KDE notifications for unfocused arrivals; click opens the pane`.

### Task 8: Window activation from a notification token (Wayland)

**Files:** Create `crates/cmux-app/src/activation.rs` (`#[cfg(target_os = "linux")]`), modify `lib.rs`.

**Interfaces:**
- Consumes: the pattern in `blur.rs`: raw handles, a guest `Backend` on gpui's `wl_display`, and its own queue.
- Produces: `pub fn activate(window: &gpui::Window, cx: &gpui::App, token: &str) -> bool`. It binds `xdg_activation_v1` (version 1), calls `activate(token, surface)`, flushes, then destroys the global. It returns false on X11 (check `cx.compositor_name()` first, as `blur.rs` does) or on any error, logging each error once. On false the caller falls back to `window.activate_window()`.

- [ ] **Step 1: Implement** `activate` following `blur.rs`: the same SAFETY reasoning, with the `wl_surface` borrowed only for the call. There is no unit test because it needs a compositor; Task 9's desktop check covers it.
- [ ] **Step 2: Wire it** into Task 7's `Activated` branch. Push and confirm Linux CI is green.
- [ ] **Step 3: Commit** `activate the window with the notification's xdg_activation token`.

### Task 9: Exit criteria and close-out

**Files:** `docs/milestones/m4-notifications/journal.md`, `README.md`, `docs/design.md`, `STATUS.md`, `docs/status-archive.md`.

- [ ] **Step 1: Desktop checks on KDE Plasma Wayland**, with results in the journal:
  - **Exit 1:** `printf '\e]9;hello\a'` in a pane of a background workspace rings that pane (visible on switching), badges the workspace and raises a KDE notification. Clicking the notification brings the window forward, even when another app is focused, and lands on the pane.
  - **Exit 2:** with three workspaces holding notifications of different ages, Super+Shift+U lands on the newest unread each time, and does nothing when none is left.
  - **Exit 3:** go through the D3 rules as a checklist against cmux on the Mac where possible:
    - a focused arrival keeps its ring until you type;
    - selecting a workspace reads only its active surface;
    - activating the window reads the focused pane;
    - closing a pane removes its notifications;
    - a flood gives a single replacing KDE bubble.
  - Also check: OSC 777 and OSC 99 (`printf '\e]99;i=1:d=0;Title\a\e]99;i=1:p=body;Body\a'`), and Claude Code's own notification hook if it emits OSC 9/777.
- [ ] **Step 2: Update the docs.**
  - README: the notification shortcuts, and that Plasma's Meta+I or Meta+Shift+U, if bound, block them.
  - `design.md`: zbus in the `cmux-app` row.
  - Journal: deviations (D7 list, D9 activation path, the Deferred list).
  - STATUS: archive first. M4 done or exit checks pending; next is M5.
- [ ] **Step 3: Commit, push, and open the PR** `M4 notifications` into `master`. Merge when CI is green and the owner agrees.

## Exit criteria and scope check

| Spec item | Task |
|---|---|
| OSC 9 / 99 / 777 parsed in cmux-term | 1 |
| Notification store in cmux-core | 2 |
| Blue pane ring, tab highlight | 5 |
| Sidebar latest text + unread badge | 6 |
| Notification panel | 6 |
| Super+Shift+U jump | 3, 4 |
| Clearing on focus as cmux does | 2 (rules), 4 (wiring) |
| KDE desktop notification via D-Bus, click focuses the pane | 7, 8 |
| Exit 1–3 | 9 |
