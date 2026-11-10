# m5-cli-socket — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-10 — Research and plan

- Started on the macOS host while the M2–M4 desktop exit checks are still pending (owner's call).
- Owner's ruling on exit criterion 1: "documented Claude Code hook setup" = Claude Code hooks in `~/.claude/settings.json` that call `cmux notify` (cmux `docs/notifications.md`). cmux's current `claude-hook`/`hooks setup` path (stdin JSON, session store, Feed) stays out of M5.
- Research: `research-cmux.md` (cmux @ 94743c7a, cited `rc §n`), from `CLI/`, `Sources/`, `Packages/`, `docs/` and the test suites; help texts copied verbatim (GPL).
- Findings that shaped the plan: the v2 protocol is cmux's own `{"id","ok","result"|"error"}` envelope, not JSON-RPC; the password goes once per connection as an `auth <pw>` line; refs are per-kind counters and bare indexes are resolved by the CLI; terminals get uppercase UUIDs in `CMUX_WORKSPACE_ID`/`CMUX_TAB_ID` and `CMUX_SURFACE_ID`/`CMUX_PANEL_ID`, and `PATH` is prefixed so `command -v cmux` finds the app's own CLI; `list-surfaces`/`focus-surface` don't exist (`list-pane-surfaces`, `focus-panel`); `notify --tab/--panel` from cmux's docs is rejected by its CLI.
- Decisions and planned deviations: plan D1–D11 (own socket path under `cmux-linux/`, cmuxOnly ancestry check via `SO_PEERCRED` + `/proc`, refs from 1, workspace-level notifications land on the focused surface, no TTY matching, help texts trimmed to what is ported).

## 2026-11-10 — Tasks 1–11 built (inline, macOS host); desktop checks pending

- Built task by task in one session (native execution), TDD for every pure piece. 87 `cmux-ipc` tests (wire, handles, JSON, snapshot, server, CLI goldens against the real server, the hook lines run through bash and the real binary) and 66 `cmux-core` tests pass; Linux CI green through `fa1468b`.
- The app could not be run here: on this macOS host gpui 0.2.2 spins in `Application::new` → `ActionRegistry::insert_action` before any cmux code (pre-existing). Tasks 9–11's "Run it" steps move to the desktop check (plan Task 12 Step 1).
- Deviations and rulings beyond the plan's D1–D11:
  - macOS gets the real ancestry check too (`LOCAL_PEERPID` + `proc_pidinfo`), not same-uid only.
  - Unknown `CMUX_SOCKET_MODE` values, cmux's `automation` among them, mean cmuxOnly.
  - Socket input (`send`, `send-key`, paste, `--command`) does not count as the user typing: it doesn't mark the pane's notification read.
  - `new-split`/`send`/`send-key` also send `CMUX_WORKSPACE_ID` as `workspace_id`, as cmux does.
  - A hook whose pane has closed (workspace still open) gets `not_found: Surface not found` instead of a ring on another pane (final review).
  - `notification.clear` for a caller that resolves to a workspace only clears that workspace.
  - Texts chosen where cmux's are UNVERIFIED: `timeout`/`Timed out waiting for the app`, `invalid_params`/`params must be an object`, `invalid_state`/`Not enough space to split`.
  - `cmux-app` gained `serde_json` (already locked; only its dependency list in `Cargo.lock` changed).
- Final review (one fresh reviewer): no Critical. Fixed the closed-pane case above; documented two silent hook failures in the README (processes outside the app's tree, such as tmux, are refused in cmuxOnly; login shells can reorder `PATH` and hide our `cmux`). Deferred minors are in STATUS.

## 2026-11-10 — Merged (PR #6); release build run on the macOS host

- Merged after Linux CI went green on `bdd7ebb`.
- The gpui hang on macOS is debug-only: a release build (`--features gpui/runtime_shaders`) starts. With it, the deferred Run-it steps of Tasks 9–11 passed against the real app over the socket (allowAll for the test):
  - The shell's env carries our `CMUX_*` ids, socket and blanked `CMUX_SOCKET`, and `command -v cmux` is ours. The login shell moved our dir far down `PATH`, but nothing earlier was named `cmux`.
  - `new-workspace` stays unselected and runs `--command`; `new-split left` puts the new pane first and keeps focus.
  - `send` and `send-key` work. `notify` (subtitle, `--desktop false`), the docs' `awk` + `dismiss-notification` line and `notify --clear` work.
  - Default mode refuses a process outside the app's tree.
  - Not seen: the visual ring, badge and panel (no screenshot taken), and KDE bubbles.
- Our CLI also works against real cmux's socket (`ping`, `current-workspace`, `list-panes`; its refs start near 10⁹), a cross-check of the wire format.
- Launched from inside a cmux terminal, the app passes that terminal's other `CMUX_*` variables (Claude hook shims, capability token) to its own shells. A local launcher script clears them; the app itself doesn't yet (M6).
