# m3-workspaces-sidebar — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-03 — Research and plan

- Branch `m3-workspaces-sidebar` starts from `m2-splits-tabs` (M2 exit checks still pending); rebase onto `master` once M2 merges.
- Research: `research-cmux.md` (cmux @ 21dcc2b Swift sources) and `research-gpui.md` (gpui 0.2.2 tarball, Zed main @ 8a47b35).
- Findings that shaped the plan: no published gpui has `ext_background_effect_v1` (Zed main added it 2026-10-30, with a platform-crate split and a wgpu renderer), so blur is a small Wayland shim on gpui's own surface; cmux's sidebar shares the terminal backdrop by default, so blur is window-wide and follows Ghostty `background-blur`; cmux gets PRs from the GitHub REST API (not `gh pr view`), and its PR rows are not state-coloured.
- Decisions: PRs via `gh pr list --json` with cmux's selection rules and cadence (spec says "via gh"); ports by controlling TTY from `/proc`; git branch by re-reading `.git/HEAD` every 1.5 s instead of watchers; Super+Shift+R renames inline (no command palette until M11).
- The plan's code was not compiled (written on a host without the Linux toolchain), unlike M2's; executors compile per task.
