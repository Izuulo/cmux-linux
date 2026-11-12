# M10 persistence — Plan

**Goal:** the three parts of the spec, one at a time. Each part is usable alone and ends with its exit criterion checked.

Branch `m10-persistence`, cut from `master`. Reference: cmux @ `88b87c05`: `Sources/SessionPersistence.swift` (sp), `SessionRestorableAgentSnapshot+Commands.swift`, `RestorableAgentHookSessionRecord.swift`, `docs/vault.md` (vd), `docs/local-tmux.md` (lt).

## Part 1: layout restore (exit 1)

| | Decision | Why |
|---|---|---|
| L1 | One file, `~/.local/state/cmux-linux/session.json` (next to `cmux.sock`), `version: 1`. Written when its content changed, checked every 8 s, and once on quit; written to a temp file and renamed. | sp `autosaveInterval` 8 s. Rename so a crash mid-write never leaves half a file. |
| L2 | Read once at start. Skipped with `CMUX_DISABLE_SESSION_RESTORE=1` or any launch argument; a file that doesn't parse is moved aside to `session.json.bad` and the app starts fresh. | sp `SessionRestorePolicy`. Keeping the bad file lets a bug be looked at instead of overwritten 8 s later. |
| L3 | Saved: window size, left sidebar width and visibility, right sidebar mode and width; workspaces in order with title and its provenance (M9 N3), the selected one, groups (name, collapsed, pinned, anchor by member index); per workspace the split tree (orientation, ratio), each pane's tabs in order, the selected tab, the focused pane; per terminal its cwd. The Dock's layout too. | sp `SessionWorkspaceSnapshot`, `SessionWorkspaceGroupSnapshot.anchorMemberIndex`. |
| L4 | Not saved: scrollback, notifications, the Feed, agent messages, window position (Wayland gives none), zoom. Ids and refs are new each run, as now. | Scope lists layout only. cmux replays scrollback through a temp file into the new shell; that is a feature of its own. |
| L5 | The tree shape and its rebuild are pure, in `cmux-core::session` (`Layout::shape()`, `Layout::from_shape()`), with tests; JSON (serde derive, already in the tree) and the timer live in the app, `window_view/session.rs`. | Same split as every model so far. |
| L6 | Restore starts each shell in its saved cwd (else `$HOME`, as now). One window, as the app has. | Our window count is one. |

| L7 | A `quit` binding, Super+Q (cmux ⌘Q), saves through gpui's quit hook. Closing the window saves too (on release). | The app had no way to quit but closing its window or its last workspace. |

Tasks: 1 shape and rebuild in `cmux-core`; 2 snapshot types and JSON; 3 save (timer, quit, atomic write); 4 restore at start; 5 check: build a layout by socket, quit, relaunch, compare `cmux list-workspaces` and `list-panes` output and a screenshot.

## Part 2: agent resume (exit 2)

| | Decision | Why |
|---|---|---|
| V1 | Claude Code reports its session through two new hooks that `cmux hooks setup` installs: `SessionStart` and `SessionEnd`, both `cmux hooks session --source claude`. It sends `surface.agent_session {surface_id, kind: "claude", session_id, cwd}`; `SessionEnd` with reason `clear`, `logout` or `prompt_input_exit` clears it, any other reason keeps it. Hook rules as M7 D3: acts only with `CMUX_LINUX=1`, prints `{}` on any error. | cmux records Claude by hook (`RestorableAgentHookSessionRecord`). A Claude killed with the app ends with reason `other`, so it stays: that is "interrupted". |
| V2 | Custom agents in `config.toml`, `[[vault.agents]]`: `id`, `name`, `process_name`, `argv_option` (where the session id is) and `resume_command` with `{{sessionId}}`, `{{executable}}`, `{{cwd}}`. Checked when saving: the terminal's foreground process (`/proc`, as the sidebar's ports) whose name (`comm`, executable or argv[0], any case, as cmux) matches gets a binding. | vd, `sessionIdSource: argvOption` only. No built-in Pi/OMP/Amp/… list, no `piSessionFile`, no `forkCommand`; `config.toml` instead of `cmux.json` (that is M11). Deviation. |
| V3 | The binding is saved with its terminal. On restore the terminal types the resume line into its new shell: `claude --resume <id>` (or the registration's command), then Enter. Session ids must match `[A-Za-z0-9._-]+`, others are dropped. | cmux auto-resumes a Claude with a valid hook binding (`shouldAutoResumeNormallyEndedClaude`). Typing reuses the Dock's `send_scripted`. The id check stops a hand-edited file from injecting shell. |
| V4 | Listed: socket `vault.list` and `cmux vault list` show every live binding (workspace, surface, agent, session id, cwd); `cmux vault resume <surface>` types the resume line again. | "Listed" in exit 2. No Vault window, search, checkpoints or fork. |

Tasks: 1 binding on the surface and `surface.agent_session`; 2 `hooks session` and `hooks setup`; 3 save and type on restore; 4 `[[vault.agents]]` and the process check; 5 `vault.list`, `cmux vault list|resume`; 6 check with a real `claude`: start, quit the app mid-turn, relaunch, see it listed and resumed.

## Part 3: local-tmux (exit 3)

| | Decision | Why |
|---|---|---|
| T1 | `cmux local-tmux start <name> [--cwd <dir>] [--command <cmd>] [--detached]`, `list`, `attach <name>`, `close <name>`. One tmux server per user on `~/.local/state/cmux-linux/local-tmux/tmux.sock` (directory mode 0700); new sessions get `history-limit 10000`; names `[A-Za-z0-9_-]{1,128}`. Needs `tmux` on `PATH` (or `CMUX_LOCAL_TMUX_BIN`). | lt "Quick start", "Limitations". |
| T2 | Attach types `TMUX= CMUX_LOCAL_TMUX=1 tmux -S <sock> attach -t <name>` into the caller's surface (`surface.send_text`); `start` without `--detached` attaches the same way. | lt: the attach line carries that marker. No new app code for attaching. |
| T3 | No registry file: tmux's own session list is the registry. When saving, a terminal whose foreground process is a `tmux … attach` on our socket with `CMUX_LOCAL_TMUX=1` in its environment gets a `local_tmux` binding; restore types the attach line again, guarded by `has-session` so a dead server prints tmux's error and leaves the shell. | lt keeps a registry with a server incarnation marker so a restarted server can't be mistaken for the old one. With names only, a new server with the same name would be attached; it is the user's own server and socket. Deviation. |
| T4 | Closing a surface detaches; only `close` kills the session. | lt. |

Not done: `status`, `detach`, `cleanup`, `--headless`, `--new-client`, the `cmux tmux attach` alias, a settings page.

Tasks: 1 CLI and tmux calls in `cmux-ipc::cli::local_tmux`; 2 detection when saving and attach on restore; 3 check: `start` with `sleep`/a counter, quit, relaunch, process still counting.

## Deferred (with reason)

- Scrollback, notifications, Feed and message persistence: outside the spec's scope list; each needs its own format and trimming.
- Multiple windows: the app has one.
- Groups' generated-anchor cleanup (M8 deferred): needs title provenance in the file, which L3 now saves; revisit after Part 1.
- Ghostty `scrollback-limit` mapping and libghostty idle compression (STATUS carry): not persistence; left for later.
