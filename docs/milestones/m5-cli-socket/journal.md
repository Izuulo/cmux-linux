# m5-cli-socket — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-10 — Research and plan

- Started on the macOS host while the M2–M4 desktop exit checks are still pending (owner's call).
- Owner's ruling on exit criterion 1: "documented Claude Code hook setup" = Claude Code hooks in `~/.claude/settings.json` that call `cmux notify` (cmux `docs/notifications.md`). cmux's current `claude-hook`/`hooks setup` path (stdin JSON, session store, Feed) stays out of M5.
- Research: `research-cmux.md` (cmux @ 94743c7a, cited `rc §n`), from `CLI/`, `Sources/`, `Packages/`, `docs/` and the test suites; help texts copied verbatim (GPL).
- Findings that shaped the plan: the v2 protocol is cmux's own `{"id","ok","result"|"error"}` envelope, not JSON-RPC; the password goes once per connection as an `auth <pw>` line; refs are per-kind counters and bare indexes are resolved by the CLI; terminals get uppercase UUIDs in `CMUX_WORKSPACE_ID`/`CMUX_TAB_ID` and `CMUX_SURFACE_ID`/`CMUX_PANEL_ID`, and `PATH` is prefixed so `command -v cmux` finds the app's own CLI; `list-surfaces`/`focus-surface` don't exist (`list-pane-surfaces`, `focus-panel`); `notify --tab/--panel` from cmux's docs is rejected by its CLI.
- Decisions and planned deviations: plan D1–D11 (own socket path under `cmux-linux/`, cmuxOnly ancestry check via `SO_PEERCRED` + `/proc`, refs from 1, workspace-level notifications land on the focused surface, no TTY matching, help texts trimmed to what is ported).
