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
- Task 3: (a) a build without `browser` keeps a saved browser tab's URL on the terminal that
  stands in for it (`Surface::kept_url`) and saves it back as the browser tab; that
  terminal's own cwd is not saved. Checked on KDE: seeded `session.json` with a browser
  tab, no-`browser` build, split + SIGTERM: the entry is still `{"kind": "browser", "url":
  ...}`. (b) Settings commits a half-typed field on app quit (`on_app_quit`) and when the
  main window goes (`WindowView` release calls `close_out` before removing Settings);
  `commit_field` lost its unused `Window` param. Not yet checked on screen (Task 7).
  (c) `.cmux-bak` is made only when none exists, so it keeps the file from before the first
  cmux-linux write ever; the per-process list is gone. Unit test extended.
- Task 4, Dock on the socket: `Snapshot::dock` (key `DOCK_KEY` 0, handle = the window's,
  as cmux gives Dock shells the window id as `CMUX_WORKSPACE_ID`; alias
  `D0CCD0CC-…0001` accepted). Found by `find_workspace`/`find_surface`/`caller`, never in
  `workspace.list`; `pane.list`/`surface.list` rows get `dock_scope: "global"`, CLI text
  `[dock:global]` (rd §9.6). App: `DOCK` id, `view_of` (Dock for 0) replaces `views.get` in
  the socket and notification paths, `select(DOCK)` shows the Dock, Dock events (OSC
  notify, focus, typing, close) go to the same handlers as a workspace's; ring and flash
  while the Dock is on screen; groups refuse it. Not done: cmux's `--placement dock` for
  `new-pane`/`new-surface`, mark-unread, a Dock unread count in the mode bar.
- Exit 3 checked on KDE: Super+Shift+E, Ctrl+5, one scripted click on the empty Dock's
  button opened a Dock terminal; in it `cmux notify --title DockHi` printed `OK
  notification:…` and a D-Bus `Notify` with DockHi went out; `cmux list-panes` printed
  `* pane:2 [1 surface] [focused]`; `surface.send_text` to it works.
- Found by that check: a scripted click right after the move missed (hit test uses the last
  drawn frame). `debug.mouse.simulate` now sends one event per frame (`dispatch_paced`).
- Task 5, CLI/socket minors, each with a test that failed first where a bug existed:
  `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC` = inf/NaN/1e300 → default 15 s (`try_from_secs_f64`);
  non-UTF-8 args → "Error: arguments must be valid UTF-8", exit 1; non-UTF-8 env vars are
  skipped. `Request::expired()`: the app skips a request its client already got a timeout
  for. `<socket>.lock` flock held by `Server` (two starts can no longer both judge an old
  socket dead and remove each other's); the app's existing fallback (own `cmux-<pid>.sock`)
  covers a held lock too. Hook stdin waits at most 3 s. `CMUX_LINUX` is now the socket path
  and hooks act only while it equals `CMUX_SOCKET_PATH` (`Cmd::in_our_terminal`); shells
  started before this build carry `1` and their hooks go quiet until reopened.
- M7 review tests added (all passed first time, no bug): invalid JSON refused by setup and
  uninstall, uninstall refuses a non-object, `HOME` fallback, `--settings-file` beats
  `CLAUDE_CONFIG_DIR`, an app error during `feed.push` prints `{}`.
- Task 6: keyboard workspace move scrolls the row into view (`scroll_to_selected`, shared
  with `select`). Long rename: the text sits in a shrinkable end-aligned box, so a name
  wider than the row loses its start, not the caret (layout not yet seen on screen). Dirty
  `*`: `DirLine::dirty` from cmux's `git status --porcelain=v1 -z --untracked-files=no
  --ignore-submodules=dirty --no-renames`, 2 s bound, `GIT_OPTIONAL_LOCKS=0`, cached per dir
  and re-run when HEAD or index mtime changed or after 10 s (cmux re-probes on git-dir
  file events; a plain edit touches neither, so we re-check on a timer too); sidebar shows
  `branch*`. `scrollback-limit` (bytes) → `Session::spawn` → `Term::with_scrollback`; a
  value Ghostty would reject (e.g. `10MB`) warns and keeps 10 MB. Applies to new terminals.
