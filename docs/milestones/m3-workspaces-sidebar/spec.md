# M3 workspaces-sidebar — Spec

**Goal:** multiple workspaces in cmux's vertical sidebar with live metadata.

## Scope
- Workspaces in `cmux-core`; new/close/rename/reorder; Super+1–8, Super+9 (last), next/prev.
- Sidebar matching cmux: workspace title, cwd (from OSC 7), git branch, listening ports, PR number/status (via `gh`, cached, rate-limited).
- `cmux-meta` collectors, each unit-tested against fixtures.
- Translucent sidebar using KWin blur.

## Out of scope
Notification text/badges in the sidebar (M4).

## Exit criteria
1. Sidebar shows correct branch/cwd/ports/PR for 5+ workspaces and updates within ~2 s of a change.
2. Visual match with the Mac sidebar (screenshot pair in journal).
3. No sidebar collector blocks the UI thread.
