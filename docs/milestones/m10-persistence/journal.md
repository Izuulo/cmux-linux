# m10-persistence — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — plan

- Read cmux @ `88b87c05`: `SessionPersistence.swift` (8 s autosave, snapshot shape, restore skipped on launch args or `CMUX_DISABLE_SESSION_RESTORE`), the Claude hook session record, `shouldAutoResumeNormallyEndedClaude`, `docs/vault.md`, `docs/local-tmux.md`. `docs/agent-recovery.md` is Subrouter's `cmux recover`, not ported.
- Wrote `plan.md`: three parts in spec order. Deviations from cmux, all in the plan: no scrollback or notification persistence; custom agents in `config.toml` with argv-option ids only; local-tmux without a registry or server incarnation marker; no Vault window.

## 2026-11-11 — Part 1, layout restore

- `cmux-core::session`: `Shape` (tree with counts, surfaces numbered in `Layout::surfaces` order), `Layout::shape`/`from_shape` (repairs zero tabs, out-of-range selection and ratios), `Workspaces::from_parts` (ids kept, next ids past the largest). App `window_view/session.rs`: JSON by hand like `dock.json` (no serde derive, none in the tree's own crates), `load`, atomic write, 8 s loop, save on quit and on release. `WorkspaceView::restore` starts each saved surface in its cwd; a surface whose shell fails is dropped.
- Plan change: no user-set tab titles exist, so none are saved. Added L7: a `quit` binding (Super+Q). The app had no quit command.
- Deviations: restored workspaces fire `workspace.created` for automations, as new ones do. A kill (SIGTERM at logout) skips the quit save; the 8 s loop bounds the loss. `CMUX_DISABLE_SESSION_RESTORE` and the launch-argument rule are cmux's.
- Check on KDE (dev build, state dir under `$XDG_RUNTIME_DIR`, socket only): three workspaces, one split right then down in `/usr/share`, two grouped and collapsed, Super+Q by `debug.shortcut.simulate`. After relaunch `list-workspaces` is the same and `workspace-group list` shows the collapsed group; `list-panes` shows the same tree order and focused pane (refs are new each run, by design). The window appeared on the owner's screen and was once clicked, which moved focus to another pane; the relaunch then restored that focus exactly too. No screenshot: the active-window capture picks up whatever the owner has in front.
- Lesson: this shell carries `CMUX_SOCKET_PATH` of the owner's app; scripted checks must set it to the dev app's socket. The scratchpad path is too long for a Unix socket.
