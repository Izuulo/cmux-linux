# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M10 persistence, branch `m10-persistence` (PR #14). Plan written 2026-11-11 (`docs/milestones/m10-persistence/plan.md`): Part 1 layout restore, Part 2 Claude/custom-agent resume, Part 3 local-tmux. No code yet. Earlier: M6–M9, the client-side window buttons and PR #13 (sidebar reopen button, rounded corners, app icon) are merged to `master`.
**Next action:** agent: M10 Part 1 Task 1 (`cmux-core::session`: `Layout::shape()`/`from_shape()` with tests), after the owner OKs the plan's deviations (auto-resume of interrupted Claude, no scrollback persistence, local-tmux without a registry). Owner, when convenient: (a) click-test PR #13's reopen button and window buttons; (b) done 2026-11-11 with Claude Code 2.1.322 (M9 journal): messages, default auto-naming, automations and a two-teammate `cmux claude-teams` run pass, the last only after adding `respawn-pane`; still open: M7 Task 9 Step 2 (a permission prompt in the Feed) and the `Stop` continue path; (c) the mouse and typing checks no script can do: M8's group header click, `+`, chevron, menus, rename field, Delete prompt, drags, sidebar edge drag, Dock "New Terminal"; older: divider/tab/row drag, Feed buttons, clicking a bubble, IME, `CMUX_TRACE_LATENCY=1` (M1 exit 3), blur by eye; (d) `makepkg -si` and a launcher start.

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (PR #1) |
| M1 terminal | done; exit criterion 1 confirmed by user, 2–3 (Ghostty screenshot pair, latency trace) not yet measured |
| M2 splits-tabs | merged (PR #3); exit checks (plan Task 11) pending at the desktop |
| M3 workspaces-sidebar | merged (PR #4); exit checks (plan Task 12 Steps 1–3) pending at the desktop |
| M4 notifications | merged (PR #5); desktop checks (plan Task 9 Step 1) pending |
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | merged (PR #7); KDE run done by script (journal 2026-11-11): perf pass, UI draws, D-Bus notification OK; owner-only checks left (install with sudo, mouse/typing, blur by eye) |
| M7 feed | merged (PR #8); Feed seen drawn on KDE (permission, question, empty state); buttons unclicked; real Claude Code run (Task 9 Step 2) pending |
| M8 groups-dock | merged (PR #10); both exit criteria checked on KDE by script and screenshot; mouse paths unverified |
| M9 agent-coordination | merged (PR #11); each exit criterion checked against the app on KDE with hand-run hooks, a fake naming command and a fake `claude`; real Claude run pending |
| M10 persistence | in progress: plan written, Part 1 next |
| M11 productivity | not started (post-v1) |

## Decisions so far
- Port from source (cmux is GPL-3.0-or-later); REA not needed. Our license: GPL-3.0-or-later.
- Never copy from cmux's BUSL-1.1 server directories.
- gpui `=0.2.2` for UI; libghostty-vt (vendored from cmux @ 21dcc2b, Ghostty fork @ ef4f157) for terminal state.
- Dev setup: Zig 0.16.0 via `scripts/install-zig.sh`; `.tools/zig` on PATH. libc pinned to 0.2.189 in Cargo.lock; never bare `cargo update`. Toolchain pinned in `rust-toolchain.toml`.
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.
- Terminal behaviour follows Ghostty source where cmux delegates to Ghostty (M1 journal lists the cases, e.g. Ctrl+[ sends CSI-u).
- Config: reads Ghostty's own config (`~/.config/ghostty/config[.ghostty]`, includes, themes).
- Model: `cmux-core` is the pure model (split tree, workspace list); contextless gpui actions bound from `config.toml`; pidfd-based shell exit (Linux).
- Notifications (M4): cmux read rules in a pure `cmux-core` store; desktop bubbles via `zbus` (already in the tree), click activates with the notification's xdg_activation token (`activation.rs`, sharing `wayland.rs` with blur).
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- M9: `cmux-core::messages` (`MessageStore`, `render_messages`); `cmux-ipc` `Snapshot::message_target`, `cli/agent.rs`, `hooks message` in `cli/hooks.rs`; app `window_view/message_ctl.rs`. Auto-naming: `cmux-config::auto_naming`, `cmux-meta::autoname` (+ `cmux_meta::sh`), `Workspaces::set_auto_title`, app `autoname_ctl.rs`. Automations: `cmux-ipc::automation`, `cli/automation.rs`, app `automation_ctl.rs` (`emit`). Claude Teams: `cmux-ipc` `cli/tmux.rs` (`claude-teams`, `__tmux-compat`), socket `surface.close`.
- M8: `cmux-core::groups` (`Group`, `Row`, `Placement`, `HeaderClick`); `cmux-ipc` `Kind::WorkspaceGroup`, `GroupSnap`, `cli/groups.rs`; app `window_view/groups.rs` (group actions), `menu.rs` (context menu), `group_ctl.rs` (socket), `dock.rs` (right sidebar frame, Dock, `dock.json`); `WorkspaceView::new_dock`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- M9: nothing ran under a real Claude Code. Messages: hook output shapes match the hooks reference; no idle wake, no off switch, not persisted. Claude Teams: the tmux commands a team sends were taken from cmux's shim, not observed; an unhandled one fails loudly; splits can fail for lack of room (no column layout). Auto-naming's default `claude -p` line is untested. Automations: a `run` that calls `cmux` can loop at its rate limit.
- M8: nothing in it was clicked or dragged (see Next action b). The Dock is invisible to the socket: `cmux notify`, the Feed hook and OSC notifications don't work from Dock terminals, and CLI commands can't target them. Dock keeps the main area's minimum pane size, so side-by-side splits need a wide sidebar. Groups and Dock are not persisted (M10).
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim runs on Plasma 6.8.2 without errors (KWin offers `ext_background_effect_manager_v1`); that the backdrop is really blurred is not yet confirmed by eye.
- M2–M7 UI has been seen drawn on KDE only through socket-driven screenshots; nothing was clicked, dragged or typed into by hand yet. On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed: cards draw; button clicks, feedback field and question picks unverified. The "sidebar wipes the terminal" report is not a bug (readline redraw when the pane is exactly prompt-wide; no OSC 133 shell integration). Real Claude Code run not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux (KDE, release): RSS 483 MB under 19 floods, CPU 0.8 % after stop.

- Agent messages (M9): app-side store, delivered only through Claude Code hooks (`hooks message` on UserPromptSubmit, PostToolUse, Stop); a workspace's agent is the surface that last claimed.
- M9 Parts 2–4: the app, not a hook, runs the naming command (`[auto_naming]`, off by default; user titles win); automations are cmux's `automations.json` with `notify` and `run` only, over six events from one `emit`; Claude Teams maps tmux pane `%N` to `surface:N` through a private `tmux` script.
- Scripted UI checks: `debug.shortcut.simulate {combo}` (raw v2 request) presses a chord; needs `CMUX_SOCKET_MODE=allowAll` from outside the app.
- Groups (M8): runs in the workspace list, anchor first, kept by `Workspaces::normalize`; the sidebar draws `rows()`; no empty groups; notification bump stays inside the group; inline group rename; a context menu instead of a palette. Dock (M8): one window-owned `WorkspaceView` in the right sidebar (mode bar Feed/Dock, draggable width), seeded by `~/.config/cmux-linux/dock.json`.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.
