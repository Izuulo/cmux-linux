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

## 2026-11-11 — Part 2, agent resume

- `cmux-config::vault`: `[[vault.agents]]` (id, name, process_name, argv_option, resume_command with `{{sessionId}}` required), `detect`, `resume_line`, `valid_session_id`. Name matching follows cmux's `executableBasenames` (the process's `comm`, the executable's file name, argv[0]'s, any case), so a Node agent that set its process title is found. `cmux-term`: `Session::foreground_pid`, `pty::process_argv` (Linux; none on macOS).
- App: an `Agent` per surface in `WorkspaceView` (hook-reported), else detected from the foreground process when saving; saved per terminal in `session.json`; on restore the resume line plus Enter is typed into the new shell. Socket `surface.agent_session` (set, or `clear` with an optional session id: an end for an older session leaves a newer one), `vault.list`, `vault.resume`. CLI `cmux vault list|resume`, `cmux hooks session --source claude`, and `hooks setup` installs `SessionStart`/`SessionEnd` with it.
- Deviation: the Claude resume line is just `claude --resume <id>`; cmux also re-applies the observed permission mode and the original launch flags.
- Check on KDE (dev build, `SHELL` a `bash --noprofile --norc` wrapper so a fake `claude` and a fake registered agent come first on `PATH`): `vault list` showed both; a `SessionEnd` with reason `other` kept Claude's; after Super+Q and relaunch the fake `claude` ran with `--resume sess-1` and the fake agent with `--session abc-7`, each in its own restored pane, and both were listed again. An end for another session id left the binding; `prompt_input_exit` cleared it. `vault resume` typed the line again; on a surface without an agent it errors.
- Mishap: a first try with the owner's zsh as the shell put `~/.local/bin` ahead of the fake, so `claude` started the real Claude Code in the dev terminal and two probe lines were typed at its start screen; Ctrl+C ended it and no session file was created. Scripted checks now use the wrapper shell.
- Still to do for exit 2: one run with a real Claude Code (owner).

## 2026-11-11 — Part 3, local-tmux

- `cmux-ipc::local_tmux`: socket path, binary (`CMUX_LOCAL_TMUX_BIN`), name rule, the attach line (`has-session` guard, then `TMUX= CMUX_LOCAL_TMUX=1 tmux -S <sock> attach-session -t =<name>`), and `detect` (a client on our socket with the marker in its environment). CLI `cmux local-tmux start|list|attach|close`; `start` sets the 10,000-line history limit and creates the session in one tmux request. Tested against a fake tmux (`tests/cli_local_tmux.rs`).
- App: an attached client is saved as an agent of kind `local-tmux` (the session name as id), found from the foreground process like a registered agent, so restore, `vault list` and `vault resume` need no separate path. The reattach line uses the configured binary, not the detected executable (a wrapper script's is its interpreter).
- Not checked live: tmux is not installed on the dev machine. A first try with a fake tmux went wrong: the launch's `env $L` did not word-split under zsh, so two dev instances started with the owner's real state directory (own `cmux-<pid>.sock` each; their autosave wrote `~/.local/state/cmux-linux/session.json`, which the installed pre-M10 build doesn't read). Neither kill nor a socket quit was possible from the agent (default access mode); left for the owner to close.
