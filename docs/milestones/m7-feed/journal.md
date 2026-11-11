# m7-feed — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — Research and plan
- Research note `research-cmux.md` from cmux @ 43ce5e2e (sparse clone, GPL dirs only).
- cmux delivers Claude's Feed through a `claude` wrapper that adds hooks via `--settings`; its settings-file installer exists only for remote hosts and carries no Feed hook. We have no wrapper, so `cmux hooks setup|uninstall` writes `~/.claude/settings.json` directly with the `PermissionRequest` Feed hook (plan D2).
- `ctrl+4` in cmux switches the right sidebar to Feed only while focus is in that sidebar; in a terminal it stays "select tab 4". Plan D7 matches that; opening the sidebar is `super+opt+b` / `super+shift+e`.
- Running our hook inside the owner's cmux.app terminal would reach cmux.app's socket (same wire format). Our terminals get `CMUX_LINUX=1` and the hook is a no-op without it (plan D3).
- Deviations planned: no Ultraplan (cmux Cloud), no Bypass (hidden for Claude in cmux too), no free-text answers, no supersede-on-later-hook (115 s timeout instead), Feed attention through M4 notifications.
- Branch `m7-feed` is cut from the unmerged `m6-parity-release`.

## 2026-11-11 — Implementation (Tasks 1–8) and Mac smoke run
Built, one line per task:
- T1 `cmux-core::feed`: `FeedStore` (newest first, cap 2000), `FeedKind`, `Decision`, modes, `age_label`.
- T2 `cmux-ipc::feed`: JSON <-> model (`parse_push`, `parse_reply`, `decision_from_json`, `hook_output`, `push_params`).
- T3 server: a push can hold its reply until the app answers (deadline per method).
- T4 `cmux hooks feed --source claude`: stdin JSON -> `feed.push` -> one stdout line, `{}` on every failure.
- T5 `cmux hooks setup|uninstall`: edits Claude's settings, only our entry, idempotent.
- T6 app: `feed_ctl.rs` holds `feed.push` until answered; ring + M4 notification; 115 s timer; close hooks.
- T7 right sidebar Feed with permission cards; bindings; `set_insets`.
- T8 plan and question cards (`feed_cards.rs`), feedback field, focus kept in the Feed when a card ends mid-edit.

Interface changes vs the plan:
- `feed_finish(request_id, FeedEnd, cx)` with `FeedEnd { Answer(Decision), TimedOut, Closed }` instead of `Option<Decision>` (R1). `Closed` answers `unavailable`.
- `Binding.context: Option<&'static str>` in `cmux-config`; only ctrl+4 has one (`RightSidebar`) (R2).
- `WorkspaceView::set_left_inset` became `set_insets(left, right)`; `toggle_right_sidebar`/`focus_right_sidebar` take `(window, cx)`.
- `FeedKind::accepts(&Decision)` in `cmux-core` (R4). `feed_refocus(window)` re-focuses the Feed after a card ends mid-edit.
- Plan and question cards live in `window_view/feed_cards.rs` (feed.rs would have passed 1000 lines); their tests moved with them. `render_right_sidebar` takes `&Window`.
- The 115 s timer is a detached `spawn_in` task, not stored; a late timer finds no waiter and does nothing.
- `Waiter.notification` is a plain `NotificationId`; `feed_close_workspace`/`feed_close_surface` are the close hooks.
- The feedback field reuses `RenameState`: single-line, no cursor movement, paste or IME; wraps instead of clipping.

Controller rulings:
- R1: `FeedEnd` enum, because workspace close must answer `{"status":"unavailable"}` and an `Option` cannot say that. Cost if wrong: a rename.
- R2: `Binding.context`, because ctrl+4 must be context-bound and `Binding` had no field for it.
- R3: `hook_output` must not panic on a non-object `tool_input` (fixed in T4, all decision kinds): the hook never errors, it prints `{}`.
- R4: a reply whose kind doesn't match the card is rejected (`invalid_params` over the socket, ignored from `feed_answer`). A wrong kind failed open toward allow (a `question.reply` on a Bash card allowed it).
- R5: reword the two ctrl+4 precedence comments (the tab handler is simply not on the sidebar's dispatch path; registration order breaks ties) and guard `on_feed_key` with `feed_focus.is_focused(window)` so the feedback field receives j/k/Enter.

Deviations from cmux's Swift app (planned in D1–D10 and kept):
- No Ultraplan (cmux Cloud), no Bypass (hidden for Claude in cmux too), no free-text answers, no supersede on a later hook (115 s timeout instead), Feed attention through M4 notifications.
- No `claude` wrapper: the hook is written into Claude's settings by `cmux hooks setup`.
- Plan text is plain, not markdown. Constants not from cmux: disabled opacity 0.4, soft fill 0.10, picked fill 0.15, option padding 8x6. A card answered keeps its text but no pick highlight.

Parked findings:
- One M4 notification per surface: two pending cards on one surface share it, so answering the newer drops the older's ring and notification; the older still answers or times out.
- `CMUX_LINUX=1` would leak into cmux.app if its binary were launched from our terminal; the installed hook would then talk to cmux.app's socket.
- The hook's stdin read has no timeout; it relies on Claude closing stdin.
- `setup`: a dangling-symlink settings file is replaced by a regular file; `--settings-file --x` takes `--x` as the path; `{"hooks":{}}` loses its `hooks` key on uninstall; if our entry isn't last, re-install rewrites and says "installed"; the marker match is a loose `contains` (plan-mandated).
- Key order in user settings is kept only because the workspace feature-unifies serde_json `preserve_order`; a `-p cmux-ipc` build alone sorts keys on change. Check how packaging builds `cmux`.
- Socket `question.reply` with fewer selections than questions is accepted (`accepts` checks the kind only); the UI gates Submit. A zero-question card keeps Submit disabled (T8's report said otherwise).
- `feed_finish` ignores the resolve/expire result, so a CAP-evicted pending item still answers resolved; R4 reads the store, not the waiter.
- Age chip refreshes only on other redraws; compact buttons at 295 px not exactly equal width; titles truncate at the end; an expired permission card has no footer; Tab inserts `\t` in the feedback field.
- A timed-out request's late reply returns `delivered:true` (matches D5). The 200 ms-deadline test has a 400 ms margin and could flake on loaded CI.
- Not covered by tests: app quitting mid-wait, CLI tests for invalid settings JSON, uninstall refusal, `HOME` fallback, `--settings-file` over `CLAUDE_CONFIG_DIR`.
- gpui `Keymap::new` hung in a unit test, so ctrl+4 precedence is checked at predicate level only.

Mac smoke run (release build, our app only, isolated state dir, hooks run in a second workspace's terminal, button clicks replaced by the matching socket reply):
- PASS: closing a workspace mid-wait ends the hook at once with `{}`; Feed keyboard (toggle/focus, j/k/Down + Enter selects the right workspace, Escape returns keys to the terminal); ctrl+4 in a terminal selects tab 4 and leaves it alone with the sidebar focused; wrong-kind replies give `invalid_params` and the card stays pending.
- PASS by socket: every permission, plan and question output is the expected JSON; the 115 s fallback prints `{}`; a 3 s wait returns `timed_out`.
- UNVERIFIED: all visuals (cards, ring, badge, dimming, `✓ Submitted`), button clicks, the feedback field, question picks and Submit gating, ctrl+4 showing the Feed. The app doesn't draw while hidden behind another full-screen app, and raising the window took the foreground from the owner.
- Possible bug, needs a manual look: opening the right sidebar seemed to wipe the visible terminal content (the screen kept one prompt line at the top). Could not check scrollback; suspect resize/reflow in `terminal_view`.
- Side notes: `XDG_STATE_HOME` leaks into child shells (test only); our zsh terminals append to the user's shell history; cmux-cua `type_text` sends only `a`, `press_key` works. A screenshot is at `~/Pictures/cmux-m7/` (sidebar open, empty).
- Step 2 (real Claude Code run) and Step 4 (PR) are not done.

## 2026-11-11 — Final review fixes
- README: Feed answers can still be given in Claude's own prompt (card stays pending until 115 s, then dims); Always Allow and All tools are identical for Claude Code; setup rewrites only when our entry changes (2-space indent, order kept); remapped `switchRightSidebarToFeed` acts only inside the sidebar.
- `terminal_env` now sets `CMUX_CLAUDE_HOOKS_DISABLED=1` (opt-out of cmux.app's `claude` wrapper, avoids a second hook and two cards).
- `FeedKind::accepts`: a question reply needs one selection per prompt. The socket-only one-string "Skip interview" reply stays accepted (constant `SKIP_INTERVIEW` in core).
- `hooks setup` on a dangling symlink writes to the link's target (relative targets resolved against the link's dir); test added.
- cmux-ipc enables serde_json `preserve_order` itself (feature flag, Cargo.lock unchanged).
- Not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON / uninstall refusal / `HOME` fallback / precedence; fake-error test for the hook; the sidebar-wipe is likely a pre-existing resize/reflow question (toggle the left sidebar to compare).
