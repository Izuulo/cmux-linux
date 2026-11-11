# m7-feed — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — Research and plan
- Research note `research-cmux.md` from cmux @ 43ce5e2e (sparse clone, GPL dirs only).
- cmux delivers Claude's Feed through a `claude` wrapper that adds hooks via `--settings`; its settings-file installer exists only for remote hosts and carries no Feed hook. We have no wrapper, so `cmux hooks setup|uninstall` writes `~/.claude/settings.json` directly with the `PermissionRequest` Feed hook (plan D2).
- `ctrl+4` in cmux switches the right sidebar to Feed only while focus is in that sidebar; in a terminal it stays "select tab 4". Plan D7 matches that; opening the sidebar is `super+opt+b` / `super+shift+e`.
- Running our hook inside the owner's cmux.app terminal would reach cmux.app's socket (same wire format). Our terminals get `CMUX_LINUX=1` and the hook is a no-op without it (plan D3).
- Deviations planned: no Ultraplan (cmux Cloud), no Bypass (hidden for Claude in cmux too), no free-text answers, no supersede-on-later-hook (115 s timeout instead), Feed attention through M4 notifications.
- Branch `m7-feed` is cut from the unmerged `m6-parity-release`.
